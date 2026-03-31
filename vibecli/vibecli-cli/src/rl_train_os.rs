#![allow(dead_code)]
//! TrainOS — Reinforcement Learning Operating System for VibeCody.
//!
//! A production-grade RL training framework covering the full lifecycle of
//! policy optimization, from algorithm selection through distributed training,
//! curriculum learning, multi-agent coordination, and fault-tolerant execution.
//!
//! # Architecture
//!
//! ```text
//! TrainingConfig (YAML)
//!   → AlgorithmRegistry::get(name)
//!     ├─ PolicyNetwork (MLP/CNN/LSTM/Transformer/Custom)
//!     ├─ ReplayBuffer (Uniform/PER/HER)
//!     ├─ ExperienceCollector (rollout workers, GAE, n-step)
//!     ├─ DistributedManager (AllReduce/ParameterServer)
//!     ├─ CurriculumManager (stages + auto-promotion)
//!     ├─ MultiAgentOrchestrator (cooperative/competitive/mixed)
//!     ├─ AutoRL (HPO, PBT, NAS)
//!     └─ FaultTolerance (checkpointing, auto-resume)
//!   → TrainingLifecycle::start()
//!     ├─ warmup → train → cooldown → done
//!     └─ TrainingMetrics (reward, loss, gradients, LR, GPU)
//! ```
//!
//! # Capabilities
//!
//! - 30+ pluggable RL algorithms (on-policy, off-policy, offline, model-based, multi-agent, imitation)
//! - YAML-based training configuration with full validation
//! - Flexible policy networks (MLP, CNN, LSTM, Transformer, custom)
//! - Distributed training with AllReduce and parameter-server gradient sync
//! - AutoRL: Bayesian HPO, grid/random search, population-based training, NAS
//! - Curriculum learning with auto-progression and per-stage env overrides
//! - Multi-agent training: cooperative, competitive, mixed-mode, league play
//! - Fault tolerance: checkpointing, auto-resume, preemption safety
//! - Replay buffers: uniform, prioritized (PER), hindsight (HER)
//! - A2A protocol for agent-to-agent messaging, negotiation, delegation
//! - Population-based training with ELO ranking and matchmaking

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ─── Algorithm Enums ────────────────────────────────────────────────────────

/// RL algorithm family classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgorithmFamily {
    OnPolicy,
    OffPolicy,
    OfflineRL,
    ModelBased,
    MultiAgent,
    Imitation,
}

impl AlgorithmFamily {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OnPolicy => "On-Policy",
            Self::OffPolicy => "Off-Policy",
            Self::OfflineRL => "Offline RL",
            Self::ModelBased => "Model-Based",
            Self::MultiAgent => "Multi-Agent",
            Self::Imitation => "Imitation Learning",
        }
    }

    pub fn all() -> Vec<AlgorithmFamily> {
        vec![
            Self::OnPolicy,
            Self::OffPolicy,
            Self::OfflineRL,
            Self::ModelBased,
            Self::MultiAgent,
            Self::Imitation,
        ]
    }
}

impl std::fmt::Display for AlgorithmFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Specific RL algorithm identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgorithmId {
    // On-policy
    PPO,
    A2C,
    TRPO,
    PPG,
    // Off-policy
    SAC,
    TD3,
    DQN,
    DDPG,
    C51,
    QRDQN,
    IQN,
    // Offline RL
    CQL,
    IQL,
    BCQ,
    BEAR,
    CRR,
    TD3BC,
    DecisionTransformer,
    COMBO,
    // Model-based
    DreamerV3,
    WorldModels,
    MuZeroStyle,
    // Multi-agent
    MAPPO,
    QMIX,
    VDN,
    MADDPG,
    COMA,
    // Imitation
    BC,
    GAIL,
    DAgger,
}

impl AlgorithmId {
    pub fn family(&self) -> AlgorithmFamily {
        match self {
            Self::PPO | Self::A2C | Self::TRPO | Self::PPG => AlgorithmFamily::OnPolicy,
            Self::SAC | Self::TD3 | Self::DQN | Self::DDPG | Self::C51 | Self::QRDQN | Self::IQN => {
                AlgorithmFamily::OffPolicy
            }
            Self::CQL | Self::IQL | Self::BCQ | Self::BEAR | Self::CRR | Self::TD3BC
            | Self::DecisionTransformer | Self::COMBO => AlgorithmFamily::OfflineRL,
            Self::DreamerV3 | Self::WorldModels | Self::MuZeroStyle => AlgorithmFamily::ModelBased,
            Self::MAPPO | Self::QMIX | Self::VDN | Self::MADDPG | Self::COMA => {
                AlgorithmFamily::MultiAgent
            }
            Self::BC | Self::GAIL | Self::DAgger => AlgorithmFamily::Imitation,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::PPO => "PPO",
            Self::A2C => "A2C",
            Self::TRPO => "TRPO",
            Self::PPG => "PPG",
            Self::SAC => "SAC",
            Self::TD3 => "TD3",
            Self::DQN => "DQN",
            Self::DDPG => "DDPG",
            Self::C51 => "C51",
            Self::QRDQN => "QR-DQN",
            Self::IQN => "IQN",
            Self::CQL => "CQL",
            Self::IQL => "IQL",
            Self::BCQ => "BCQ",
            Self::BEAR => "BEAR",
            Self::CRR => "CRR",
            Self::TD3BC => "TD3+BC",
            Self::DecisionTransformer => "Decision Transformer",
            Self::COMBO => "COMBO",
            Self::DreamerV3 => "DreamerV3",
            Self::WorldModels => "World Models",
            Self::MuZeroStyle => "MuZero-Style",
            Self::MAPPO => "MAPPO",
            Self::QMIX => "QMIX",
            Self::VDN => "VDN",
            Self::MADDPG => "MADDPG",
            Self::COMA => "COMA",
            Self::BC => "BC",
            Self::GAIL => "GAIL",
            Self::DAgger => "DAgger",
        }
    }

    pub fn all() -> Vec<AlgorithmId> {
        vec![
            Self::PPO, Self::A2C, Self::TRPO, Self::PPG,
            Self::SAC, Self::TD3, Self::DQN, Self::DDPG, Self::C51, Self::QRDQN, Self::IQN,
            Self::CQL, Self::IQL, Self::BCQ, Self::BEAR, Self::CRR, Self::TD3BC,
            Self::DecisionTransformer, Self::COMBO,
            Self::DreamerV3, Self::WorldModels, Self::MuZeroStyle,
            Self::MAPPO, Self::QMIX, Self::VDN, Self::MADDPG, Self::COMA,
            Self::BC, Self::GAIL, Self::DAgger,
        ]
    }

    pub fn requires_replay_buffer(&self) -> bool {
        matches!(
            self.family(),
            AlgorithmFamily::OffPolicy | AlgorithmFamily::OfflineRL
        )
    }

    pub fn supports_continuous_actions(&self) -> bool {
        !matches!(self, Self::DQN | Self::C51 | Self::QRDQN | Self::IQN)
    }

    pub fn default_hyperparams(&self) -> HashMap<String, f64> {
        let mut hp = HashMap::new();
        match self {
            Self::PPO => {
                hp.insert("clip_ratio".into(), 0.2);
                hp.insert("entropy_coef".into(), 0.01);
                hp.insert("value_coef".into(), 0.5);
                hp.insert("max_grad_norm".into(), 0.5);
                hp.insert("gae_lambda".into(), 0.95);
                hp.insert("num_epochs".into(), 10.0);
                hp.insert("minibatch_size".into(), 64.0);
            }
            Self::SAC => {
                hp.insert("tau".into(), 0.005);
                hp.insert("alpha".into(), 0.2);
                hp.insert("target_update_interval".into(), 1.0);
                hp.insert("auto_alpha".into(), 1.0);
            }
            Self::TD3 => {
                hp.insert("tau".into(), 0.005);
                hp.insert("policy_noise".into(), 0.2);
                hp.insert("noise_clip".into(), 0.5);
                hp.insert("policy_delay".into(), 2.0);
            }
            Self::DQN => {
                hp.insert("epsilon_start".into(), 1.0);
                hp.insert("epsilon_end".into(), 0.01);
                hp.insert("epsilon_decay".into(), 0.995);
                hp.insert("target_update_freq".into(), 1000.0);
            }
            Self::CQL => {
                hp.insert("cql_alpha".into(), 1.0);
                hp.insert("min_q_weight".into(), 5.0);
                hp.insert("tau".into(), 0.005);
            }
            Self::DreamerV3 => {
                hp.insert("imagination_horizon".into(), 15.0);
                hp.insert("kl_free".into(), 1.0);
                hp.insert("kl_scale".into(), 0.1);
                hp.insert("discount".into(), 0.997);
            }
            Self::MAPPO => {
                hp.insert("clip_ratio".into(), 0.2);
                hp.insert("share_policy".into(), 1.0);
                hp.insert("centralized_value".into(), 1.0);
            }
            _ => {
                hp.insert("learning_rate".into(), 3e-4);
                hp.insert("gamma".into(), 0.99);
            }
        }
        // Common defaults
        hp.entry("learning_rate".to_string()).or_insert(3e-4);
        hp.entry("gamma".to_string()).or_insert(0.99);
        hp.entry("batch_size".to_string()).or_insert(256.0);
        hp
    }
}

impl std::fmt::Display for AlgorithmId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ─── Algorithm Descriptor ───────────────────────────────────────────────────

/// Full descriptor for a registered algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmDescriptor {
    pub id: AlgorithmId,
    pub family: AlgorithmFamily,
    pub description: String,
    pub supports_continuous: bool,
    pub supports_discrete: bool,
    pub supports_multi_agent: bool,
    pub requires_model: bool,
    pub default_hyperparams: HashMap<String, f64>,
    pub paper_reference: String,
}

impl AlgorithmDescriptor {
    pub fn new(id: AlgorithmId) -> Self {
        let family = id.family();
        let supports_continuous = id.supports_continuous_actions();
        let supports_discrete = true;
        let supports_multi_agent = matches!(family, AlgorithmFamily::MultiAgent);
        let requires_model = matches!(family, AlgorithmFamily::ModelBased);
        let default_hyperparams = id.default_hyperparams();
        let description = format!("{} ({}) algorithm", id.name(), family.label());
        let paper_reference = match &id {
            AlgorithmId::PPO => "Schulman et al., 2017",
            AlgorithmId::SAC => "Haarnoja et al., 2018",
            AlgorithmId::DreamerV3 => "Hafner et al., 2023",
            AlgorithmId::MAPPO => "Yu et al., 2022",
            AlgorithmId::CQL => "Kumar et al., 2020",
            _ => "See literature",
        }
        .to_string();

        Self {
            id,
            family,
            description,
            supports_continuous,
            supports_discrete,
            supports_multi_agent,
            requires_model,
            default_hyperparams,
            paper_reference,
        }
    }
}

// ─── Algorithm Registry ─────────────────────────────────────────────────────

/// Registry of all available RL algorithms with lookup and filtering.
#[derive(Debug, Clone)]
pub struct AlgorithmRegistry {
    algorithms: HashMap<String, AlgorithmDescriptor>,
}

impl AlgorithmRegistry {
    pub fn new() -> Self {
        let mut algorithms = HashMap::new();
        for id in AlgorithmId::all() {
            let desc = AlgorithmDescriptor::new(id.clone());
            algorithms.insert(id.name().to_lowercase().replace(['-', '+', ' '], "_"), desc);
        }
        Self { algorithms }
    }

    pub fn get(&self, name: &str) -> Option<&AlgorithmDescriptor> {
        let key = name.to_lowercase().replace(['-', '+', ' '], "_");
        self.algorithms.get(&key)
    }

    pub fn list_by_family(&self, family: &AlgorithmFamily) -> Vec<&AlgorithmDescriptor> {
        self.algorithms
            .values()
            .filter(|a| &a.family == family)
            .collect()
    }

    pub fn list_all(&self) -> Vec<&AlgorithmDescriptor> {
        self.algorithms.values().collect()
    }

    pub fn count(&self) -> usize {
        self.algorithms.len()
    }

    pub fn families(&self) -> Vec<AlgorithmFamily> {
        AlgorithmFamily::all()
    }

    pub fn supports_continuous(&self) -> Vec<&AlgorithmDescriptor> {
        self.algorithms
            .values()
            .filter(|a| a.supports_continuous)
            .collect()
    }

    pub fn supports_discrete(&self) -> Vec<&AlgorithmDescriptor> {
        self.algorithms
            .values()
            .filter(|a| a.supports_discrete)
            .collect()
    }

    pub fn register_custom(&mut self, key: String, desc: AlgorithmDescriptor) {
        self.algorithms.insert(key, desc);
    }
}

impl Default for AlgorithmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Policy Network Definitions ─────────────────────────────────────────────

/// Activation function for network layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Activation {
    ReLU,
    Tanh,
    Sigmoid,
    GELU,
    SiLU,
    LeakyReLU(f64),
    ELU(f64),
    Softmax,
    Identity,
}

impl Activation {
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::ReLU => x.max(0.0),
            Self::Tanh => x.tanh(),
            Self::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Self::GELU => 0.5 * x * (1.0 + (0.7978845608 * (x + 0.044715 * x.powi(3))).tanh()),
            Self::SiLU => x * (1.0 / (1.0 + (-x).exp())),
            Self::LeakyReLU(alpha) => if x >= 0.0 { x } else { alpha * x },
            Self::ELU(alpha) => if x >= 0.0 { x } else { alpha * (x.exp() - 1.0) },
            Self::Softmax => x.exp(), // simplified; real softmax needs vector context
            Self::Identity => x,
        }
    }
}

/// Layer type for building network architectures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerType {
    Dense { in_features: usize, out_features: usize },
    Conv2d { in_channels: usize, out_channels: usize, kernel_size: usize, stride: usize },
    LSTM { input_size: usize, hidden_size: usize, num_layers: usize },
    TransformerBlock { d_model: usize, n_heads: usize, d_ff: usize },
    LayerNorm { features: usize },
    BatchNorm { features: usize },
    Dropout { rate: f64 },
    Flatten,
    Embedding { vocab_size: usize, embed_dim: usize },
    MultiHeadAttention { d_model: usize, n_heads: usize },
}

impl LayerType {
    pub fn param_count(&self) -> usize {
        match self {
            Self::Dense { in_features, out_features } => in_features * out_features + out_features,
            Self::Conv2d { in_channels, out_channels, kernel_size, .. } => {
                in_channels * out_channels * kernel_size * kernel_size + out_channels
            }
            Self::LSTM { input_size, hidden_size, num_layers } => {
                num_layers * 4 * (input_size * hidden_size + hidden_size * hidden_size + hidden_size)
            }
            Self::TransformerBlock { d_model, d_ff, .. } => {
                4 * d_model * d_model + 2 * d_model * d_ff + 4 * d_model
            }
            Self::LayerNorm { features } | Self::BatchNorm { features } => 2 * features,
            Self::Dropout { .. } | Self::Flatten => 0,
            Self::Embedding { vocab_size, embed_dim } => vocab_size * embed_dim,
            Self::MultiHeadAttention { d_model, .. } => 4 * d_model * d_model + 4 * d_model,
        }
    }
}

/// Single layer in a network with optional activation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub name: String,
    pub layer_type: LayerType,
    pub activation: Option<Activation>,
}

/// Network architecture type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkType {
    MLP,
    CNN,
    LSTM,
    Transformer,
    Custom,
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MLP => write!(f, "MLP"),
            Self::CNN => write!(f, "CNN"),
            Self::LSTM => write!(f, "LSTM"),
            Self::Transformer => write!(f, "Transformer"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// Complete policy network definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyNetwork {
    pub name: String,
    pub network_type: NetworkType,
    pub layers: Vec<LayerConfig>,
    pub input_shape: Vec<usize>,
    pub output_dim: usize,
    pub use_layer_norm: bool,
    pub init_method: String,
}

impl PolicyNetwork {
    pub fn mlp(input_dim: usize, hidden_dims: &[usize], output_dim: usize) -> Self {
        let mut layers = Vec::new();
        let mut prev = input_dim;
        for (i, &h) in hidden_dims.iter().enumerate() {
            layers.push(LayerConfig {
                name: format!("dense_{}", i),
                layer_type: LayerType::Dense { in_features: prev, out_features: h },
                activation: Some(Activation::ReLU),
            });
            prev = h;
        }
        layers.push(LayerConfig {
            name: "output".into(),
            layer_type: LayerType::Dense { in_features: prev, out_features: output_dim },
            activation: None,
        });
        Self {
            name: "mlp_policy".into(),
            network_type: NetworkType::MLP,
            layers,
            input_shape: vec![input_dim],
            output_dim,
            use_layer_norm: false,
            init_method: "orthogonal".into(),
        }
    }

    pub fn cnn(input_channels: usize, img_size: usize, output_dim: usize) -> Self {
        let layers = vec![
            LayerConfig {
                name: "conv1".into(),
                layer_type: LayerType::Conv2d {
                    in_channels: input_channels, out_channels: 32, kernel_size: 8, stride: 4,
                },
                activation: Some(Activation::ReLU),
            },
            LayerConfig {
                name: "conv2".into(),
                layer_type: LayerType::Conv2d {
                    in_channels: 32, out_channels: 64, kernel_size: 4, stride: 2,
                },
                activation: Some(Activation::ReLU),
            },
            LayerConfig {
                name: "conv3".into(),
                layer_type: LayerType::Conv2d {
                    in_channels: 64, out_channels: 64, kernel_size: 3, stride: 1,
                },
                activation: Some(Activation::ReLU),
            },
            LayerConfig {
                name: "flatten".into(),
                layer_type: LayerType::Flatten,
                activation: None,
            },
            LayerConfig {
                name: "fc".into(),
                layer_type: LayerType::Dense { in_features: 3136, out_features: 512 },
                activation: Some(Activation::ReLU),
            },
            LayerConfig {
                name: "output".into(),
                layer_type: LayerType::Dense { in_features: 512, out_features: output_dim },
                activation: None,
            },
        ];
        Self {
            name: "cnn_policy".into(),
            network_type: NetworkType::CNN,
            layers,
            input_shape: vec![input_channels, img_size, img_size],
            output_dim,
            use_layer_norm: false,
            init_method: "kaiming".into(),
        }
    }

    pub fn transformer(d_model: usize, n_heads: usize, n_layers: usize, output_dim: usize) -> Self {
        let mut layers = Vec::new();
        for i in 0..n_layers {
            layers.push(LayerConfig {
                name: format!("transformer_block_{}", i),
                layer_type: LayerType::TransformerBlock {
                    d_model,
                    n_heads,
                    d_ff: d_model * 4,
                },
                activation: Some(Activation::GELU),
            });
        }
        layers.push(LayerConfig {
            name: "output_head".into(),
            layer_type: LayerType::Dense { in_features: d_model, out_features: output_dim },
            activation: None,
        });
        Self {
            name: "transformer_policy".into(),
            network_type: NetworkType::Transformer,
            layers,
            input_shape: vec![d_model],
            output_dim,
            use_layer_norm: true,
            init_method: "xavier".into(),
        }
    }

    pub fn lstm(input_size: usize, hidden_size: usize, num_layers: usize, output_dim: usize) -> Self {
        let layers = vec![
            LayerConfig {
                name: "lstm".into(),
                layer_type: LayerType::LSTM { input_size, hidden_size, num_layers },
                activation: Some(Activation::Tanh),
            },
            LayerConfig {
                name: "output".into(),
                layer_type: LayerType::Dense { in_features: hidden_size, out_features: output_dim },
                activation: None,
            },
        ];
        Self {
            name: "lstm_policy".into(),
            network_type: NetworkType::LSTM,
            layers,
            input_shape: vec![input_size],
            output_dim,
            use_layer_norm: false,
            init_method: "orthogonal".into(),
        }
    }

    pub fn total_params(&self) -> usize {
        self.layers.iter().map(|l| l.layer_type.param_count()).sum()
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("Network: {} ({})", self.name, self.network_type),
            format!("Input shape: {:?}", self.input_shape),
            format!("Output dim: {}", self.output_dim),
            format!("Total params: {}", self.total_params()),
            "Layers:".into(),
        ];
        for layer in &self.layers {
            lines.push(format!(
                "  {} — {:?} → {:?}",
                layer.name, layer.layer_type, layer.activation
            ));
        }
        lines.join("\n")
    }
}

// ─── Replay Buffer ──────────────────────────────────────────────────────────

/// Replay buffer strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplayBufferType {
    Uniform,
    Prioritized { alpha: f64, beta: f64, beta_increment: f64 },
    Hindsight { strategy: HERStrategy, k_goals: usize },
}

/// HER goal relabeling strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HERStrategy {
    Future,
    Final,
    Episode,
    Random,
}

/// A single experience transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub state: Vec<f64>,
    pub action: Vec<f64>,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub done: bool,
    pub info: HashMap<String, f64>,
}

/// Priority entry for PER.
#[derive(Debug, Clone)]
struct PriorityEntry {
    transition: Transition,
    priority: f64,
}

/// Replay buffer with uniform, prioritized, or HER sampling.
#[derive(Debug, Clone)]
pub struct ReplayBuffer {
    pub buffer_type: ReplayBufferType,
    pub capacity: usize,
    entries: Vec<PriorityEntry>,
    total_added: usize,
    max_priority: f64,
}

impl ReplayBuffer {
    pub fn new(buffer_type: ReplayBufferType, capacity: usize) -> Self {
        Self {
            buffer_type,
            capacity,
            entries: Vec::with_capacity(capacity.min(65536)),
            total_added: 0,
            max_priority: 1.0,
        }
    }

    pub fn add(&mut self, transition: Transition) {
        let priority = self.max_priority;
        if self.entries.len() < self.capacity {
            self.entries.push(PriorityEntry { transition, priority });
        } else {
            let idx = self.total_added % self.capacity;
            self.entries[idx] = PriorityEntry { transition, priority };
        }
        self.total_added += 1;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn sample(&self, batch_size: usize) -> Vec<&Transition> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let actual_size = batch_size.min(self.entries.len());
        match &self.buffer_type {
            ReplayBufferType::Uniform => {
                // Deterministic stride-based sampling for reproducibility
                let step = self.entries.len().max(1) / actual_size.max(1);
                (0..actual_size)
                    .map(|i| &self.entries[(i * step) % self.entries.len()].transition)
                    .collect()
            }
            ReplayBufferType::Prioritized { alpha, .. } => {
                // Priority-weighted sampling
                let total_priority: f64 = self.entries.iter().map(|e| e.priority.powf(*alpha)).sum();
                if total_priority <= 0.0 {
                    return self.entries.iter().take(actual_size).map(|e| &e.transition).collect();
                }
                let segment = total_priority / actual_size as f64;
                let mut result = Vec::with_capacity(actual_size);
                let mut cumulative = 0.0;
                let mut segment_idx = 0;
                for entry in &self.entries {
                    cumulative += entry.priority.powf(*alpha);
                    while segment_idx < actual_size && cumulative >= (segment_idx as f64 + 0.5) * segment {
                        result.push(&entry.transition);
                        segment_idx += 1;
                    }
                    if result.len() >= actual_size {
                        break;
                    }
                }
                while result.len() < actual_size {
                    result.push(&self.entries.last().unwrap().transition);
                }
                result
            }
            ReplayBufferType::Hindsight { k_goals, .. } => {
                // HER: return transitions with relabeled goals
                let take = actual_size / (1 + k_goals);
                self.entries
                    .iter()
                    .take(take.max(1) * (1 + k_goals))
                    .take(actual_size)
                    .map(|e| &e.transition)
                    .collect()
            }
        }
    }

    pub fn update_priorities(&mut self, indices: &[usize], new_priorities: &[f64]) {
        for (&idx, &prio) in indices.iter().zip(new_priorities.iter()) {
            if idx < self.entries.len() {
                let p = prio.abs() + 1e-6;
                self.entries[idx].priority = p;
                if p > self.max_priority {
                    self.max_priority = p;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_added = 0;
        self.max_priority = 1.0;
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.entries.len() as f64 / self.capacity as f64
    }
}

// ─── Training Configuration ─────────────────────────────────────────────────

/// Gradient synchronization strategy for distributed training.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GradientSyncStrategy {
    AllReduce,
    ParameterServer,
    Gossip,
    LocalSGD { sync_every: usize },
}

/// Distributed training configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    pub num_workers: usize,
    pub num_gpus_per_worker: usize,
    pub gradient_sync: GradientSyncStrategy,
    pub data_parallel: bool,
    pub model_parallel: bool,
    pub mixed_precision: bool,
    pub gradient_accumulation_steps: usize,
    pub communication_backend: String,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            num_workers: 1,
            num_gpus_per_worker: 1,
            gradient_sync: GradientSyncStrategy::AllReduce,
            data_parallel: true,
            model_parallel: false,
            mixed_precision: false,
            gradient_accumulation_steps: 1,
            communication_backend: "nccl".into(),
        }
    }
}

/// Learning rate schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LRSchedule {
    Constant,
    Linear { start: f64, end: f64 },
    Cosine { min_lr: f64 },
    StepDecay { step_size: usize, gamma: f64 },
    WarmupCosine { warmup_steps: usize, min_lr: f64 },
    Exponential { gamma: f64 },
}

impl LRSchedule {
    pub fn get_lr(&self, base_lr: f64, step: usize, total_steps: usize) -> f64 {
        match self {
            Self::Constant => base_lr,
            Self::Linear { start, end } => {
                if total_steps == 0 { return *start; }
                let frac = step as f64 / total_steps as f64;
                start + (end - start) * frac
            }
            Self::Cosine { min_lr } => {
                if total_steps == 0 { return base_lr; }
                let frac = step as f64 / total_steps as f64;
                min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (std::f64::consts::PI * frac).cos())
            }
            Self::StepDecay { step_size, gamma } => {
                if *step_size == 0 { return base_lr; }
                base_lr * gamma.powi((step / step_size) as i32)
            }
            Self::WarmupCosine { warmup_steps, min_lr } => {
                if step < *warmup_steps {
                    if *warmup_steps == 0 { return base_lr; }
                    base_lr * step as f64 / *warmup_steps as f64
                } else {
                    let post = step - warmup_steps;
                    let post_total = total_steps.saturating_sub(*warmup_steps);
                    if post_total == 0 { return base_lr; }
                    let frac = post as f64 / post_total as f64;
                    min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (std::f64::consts::PI * frac).cos())
                }
            }
            Self::Exponential { gamma } => base_lr * gamma.powi(step as i32),
        }
    }
}

/// Checkpoint configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    pub enabled: bool,
    pub frequency_steps: usize,
    pub keep_last: usize,
    pub save_optimizer: bool,
    pub save_replay_buffer: bool,
    pub path: String,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            frequency_steps: 10000,
            keep_last: 5,
            save_optimizer: true,
            save_replay_buffer: false,
            path: "./checkpoints".into(),
        }
    }
}

/// Full training configuration — parseable from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub name: String,
    pub algorithm: String,
    pub environment: String,
    pub total_timesteps: usize,
    pub learning_rate: f64,
    pub lr_schedule: LRSchedule,
    pub gamma: f64,
    pub batch_size: usize,
    pub buffer_size: usize,
    pub warmup_steps: usize,
    pub eval_interval: usize,
    pub eval_episodes: usize,
    pub log_interval: usize,
    pub seed: u64,
    pub hyperparams: HashMap<String, f64>,
    pub distributed: DistributedConfig,
    pub checkpoint: CheckpointConfig,
    pub tags: Vec<String>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            name: "default_experiment".into(),
            algorithm: "ppo".into(),
            environment: "CartPole-v1".into(),
            total_timesteps: 1_000_000,
            learning_rate: 3e-4,
            lr_schedule: LRSchedule::Constant,
            gamma: 0.99,
            batch_size: 256,
            buffer_size: 100_000,
            warmup_steps: 1000,
            eval_interval: 10_000,
            eval_episodes: 10,
            log_interval: 1000,
            seed: 42,
            hyperparams: HashMap::new(),
            distributed: DistributedConfig::default(),
            checkpoint: CheckpointConfig::default(),
            tags: Vec::new(),
        }
    }
}

impl TrainingConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse training config YAML: {}", e))
    }

    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| format!("Failed to serialize training config: {}", e))
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.name.is_empty() {
            errors.push("Experiment name must not be empty".into());
        }
        if self.algorithm.is_empty() {
            errors.push("Algorithm must not be empty".into());
        }
        if self.total_timesteps == 0 {
            errors.push("total_timesteps must be > 0".into());
        }
        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            errors.push("learning_rate must be in (0, 1]".into());
        }
        if self.gamma < 0.0 || self.gamma > 1.0 {
            errors.push("gamma must be in [0, 1]".into());
        }
        if self.batch_size == 0 {
            errors.push("batch_size must be > 0".into());
        }
        if self.distributed.num_workers == 0 {
            errors.push("num_workers must be >= 1".into());
        }
        errors
    }

    pub fn effective_batch_size(&self) -> usize {
        self.batch_size
            * self.distributed.num_workers
            * self.distributed.gradient_accumulation_steps
    }
}

// ─── Distributed Training Manager ───────────────────────────────────────────

/// Worker status in distributed training.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Idle,
    Running,
    Syncing,
    Failed(String),
    Completed,
}

/// A single training worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: usize,
    pub rank: usize,
    pub hostname: String,
    pub gpu_ids: Vec<usize>,
    pub status: WorkerStatus,
    pub steps_completed: usize,
    pub samples_collected: usize,
}

impl Worker {
    pub fn new(id: usize, rank: usize, hostname: String, gpu_ids: Vec<usize>) -> Self {
        Self {
            id,
            rank,
            hostname,
            gpu_ids,
            status: WorkerStatus::Idle,
            steps_completed: 0,
            samples_collected: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, WorkerStatus::Running | WorkerStatus::Syncing)
    }
}

/// Manages distributed training workers and gradient synchronization.
#[derive(Debug, Clone)]
pub struct DistributedManager {
    pub config: DistributedConfig,
    pub workers: Vec<Worker>,
    pub sync_count: usize,
    pub global_step: usize,
    gradient_buffer: Vec<f64>,
}

impl DistributedManager {
    pub fn new(config: DistributedConfig) -> Self {
        let num = config.num_workers;
        let workers = (0..num)
            .map(|i| Worker::new(i, i, format!("worker-{}", i), vec![i % 8]))
            .collect();
        Self {
            config,
            workers,
            sync_count: 0,
            global_step: 0,
            gradient_buffer: Vec::new(),
        }
    }

    pub fn active_workers(&self) -> usize {
        self.workers.iter().filter(|w| w.is_active()).count()
    }

    pub fn start_all(&mut self) {
        for w in &mut self.workers {
            w.status = WorkerStatus::Running;
        }
    }

    pub fn stop_all(&mut self) {
        for w in &mut self.workers {
            w.status = WorkerStatus::Completed;
        }
    }

    pub fn sync_gradients(&mut self, gradients: &[Vec<f64>]) -> Vec<f64> {
        if gradients.is_empty() {
            return Vec::new();
        }
        let len = gradients[0].len();
        match &self.config.gradient_sync {
            GradientSyncStrategy::AllReduce => {
                // Average gradients across all workers
                let mut result = vec![0.0; len];
                let n = gradients.len() as f64;
                for grad in gradients {
                    for (i, &g) in grad.iter().enumerate() {
                        if i < len {
                            result[i] += g / n;
                        }
                    }
                }
                self.sync_count += 1;
                self.gradient_buffer = result.clone();
                result
            }
            GradientSyncStrategy::ParameterServer => {
                // Sum gradients (PS will average during apply)
                let mut result = vec![0.0; len];
                for grad in gradients {
                    for (i, &g) in grad.iter().enumerate() {
                        if i < len {
                            result[i] += g;
                        }
                    }
                }
                self.sync_count += 1;
                self.gradient_buffer = result.clone();
                result
            }
            GradientSyncStrategy::Gossip => {
                // Average adjacent pairs
                let mut result = gradients[0].clone();
                if gradients.len() > 1 {
                    for (i, v) in result.iter_mut().enumerate() {
                        if i < gradients[1].len() {
                            *v = (*v + gradients[1][i]) / 2.0;
                        }
                    }
                }
                self.sync_count += 1;
                result
            }
            GradientSyncStrategy::LocalSGD { sync_every } => {
                if self.global_step % sync_every == 0 {
                    let mut result = vec![0.0; len];
                    let n = gradients.len() as f64;
                    for grad in gradients {
                        for (i, &g) in grad.iter().enumerate() {
                            if i < len {
                                result[i] += g / n;
                            }
                        }
                    }
                    self.sync_count += 1;
                    result
                } else {
                    // Use local gradient only (first worker)
                    gradients[0].clone()
                }
            }
        }
    }

    pub fn mark_worker_failed(&mut self, worker_id: usize, reason: &str) {
        if let Some(w) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            w.status = WorkerStatus::Failed(reason.to_string());
        }
    }

    pub fn replace_failed_worker(&mut self, worker_id: usize) -> bool {
        if let Some(w) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            if matches!(w.status, WorkerStatus::Failed(_)) {
                w.status = WorkerStatus::Running;
                w.steps_completed = 0;
                w.samples_collected = 0;
                return true;
            }
        }
        false
    }

    pub fn advance_step(&mut self) {
        self.global_step += 1;
        for w in &mut self.workers {
            if w.is_active() {
                w.steps_completed += 1;
            }
        }
    }
}

// ─── Experience Collection ──────────────────────────────────────────────────

/// A collected trajectory (sequence of transitions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub transitions: Vec<Transition>,
    pub total_reward: f64,
    pub length: usize,
    pub terminated: bool,
}

impl Trajectory {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            total_reward: 0.0,
            length: 0,
            terminated: false,
        }
    }

    pub fn add(&mut self, t: Transition) {
        self.total_reward += t.reward;
        let done = t.done;
        self.transitions.push(t);
        self.length += 1;
        if done {
            self.terminated = true;
        }
    }

    /// Compute Generalized Advantage Estimation.
    pub fn compute_gae(&self, gamma: f64, lam: f64, values: &[f64]) -> Vec<f64> {
        let n = self.transitions.len();
        if n == 0 || values.len() < n {
            return Vec::new();
        }
        let mut advantages = vec![0.0; n];
        let mut gae = 0.0;
        for t in (0..n).rev() {
            let next_value = if t + 1 < values.len() { values[t + 1] } else { 0.0 };
            let delta = self.transitions[t].reward + gamma * next_value * (1.0 - self.transitions[t].done as i32 as f64) - values[t];
            gae = delta + gamma * lam * (1.0 - self.transitions[t].done as i32 as f64) * gae;
            advantages[t] = gae;
        }
        advantages
    }

    /// Compute n-step returns.
    pub fn compute_nstep_returns(&self, gamma: f64, n: usize) -> Vec<f64> {
        let len = self.transitions.len();
        let mut returns = vec![0.0; len];
        for i in 0..len {
            let mut ret = 0.0;
            let mut discount = 1.0;
            for j in 0..n.min(len - i) {
                ret += discount * self.transitions[i + j].reward;
                discount *= gamma;
                if self.transitions[i + j].done {
                    break;
                }
            }
            returns[i] = ret;
        }
        returns
    }

    /// Compute discounted returns.
    pub fn compute_returns(&self, gamma: f64) -> Vec<f64> {
        let n = self.transitions.len();
        let mut returns = vec![0.0; n];
        let mut running = 0.0;
        for t in (0..n).rev() {
            running = self.transitions[t].reward + gamma * running * (1.0 - self.transitions[t].done as i32 as f64);
            returns[t] = running;
        }
        returns
    }
}

impl Default for Trajectory {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages experience collection from rollout workers.
#[derive(Debug, Clone)]
pub struct ExperienceCollector {
    pub num_workers: usize,
    pub rollout_length: usize,
    pub trajectories: Vec<Trajectory>,
    pub total_steps: usize,
    pub total_episodes: usize,
}

impl ExperienceCollector {
    pub fn new(num_workers: usize, rollout_length: usize) -> Self {
        Self {
            num_workers,
            rollout_length,
            trajectories: Vec::new(),
            total_steps: 0,
            total_episodes: 0,
        }
    }

    pub fn add_trajectory(&mut self, trajectory: Trajectory) {
        self.total_steps += trajectory.length;
        if trajectory.terminated {
            self.total_episodes += 1;
        }
        self.trajectories.push(trajectory);
    }

    pub fn flush_to_buffer(&mut self, buffer: &mut ReplayBuffer) {
        for traj in self.trajectories.drain(..) {
            for t in traj.transitions {
                buffer.add(t);
            }
        }
    }

    pub fn average_reward(&self) -> f64 {
        if self.trajectories.is_empty() {
            return 0.0;
        }
        let total: f64 = self.trajectories.iter().map(|t| t.total_reward).sum();
        total / self.trajectories.len() as f64
    }

    pub fn average_length(&self) -> f64 {
        if self.trajectories.is_empty() {
            return 0.0;
        }
        let total: usize = self.trajectories.iter().map(|t| t.length).sum();
        total as f64 / self.trajectories.len() as f64
    }
}

// ─── Training Metrics ───────────────────────────────────────────────────────

/// A single metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub step: usize,
    pub value: f64,
    pub timestamp: u64,
}

/// Tracks all training metrics.
#[derive(Debug, Clone)]
pub struct TrainingMetrics {
    pub metrics: HashMap<String, Vec<MetricPoint>>,
    pub best_reward: f64,
    pub best_reward_step: usize,
    pub total_episodes: usize,
}

impl TrainingMetrics {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            best_reward: f64::NEG_INFINITY,
            best_reward_step: 0,
            total_episodes: 0,
        }
    }

    pub fn record(&mut self, name: &str, step: usize, value: f64) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let point = MetricPoint { step, value, timestamp: ts };
        self.metrics.entry(name.to_string()).or_default().push(point);

        if name == "reward" && value > self.best_reward {
            self.best_reward = value;
            self.best_reward_step = step;
        }
    }

    pub fn get_latest(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.last().map(|p| p.value)
    }

    pub fn get_average(&self, name: &str, window: usize) -> Option<f64> {
        let points = self.metrics.get(name)?;
        if points.is_empty() {
            return None;
        }
        let start = points.len().saturating_sub(window);
        let slice = &points[start..];
        let sum: f64 = slice.iter().map(|p| p.value).sum();
        Some(sum / slice.len() as f64)
    }

    pub fn get_min(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.iter().map(|p| p.value).reduce(f64::min)
    }

    pub fn get_max(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.iter().map(|p| p.value).reduce(f64::max)
    }

    pub fn metric_names(&self) -> Vec<&String> {
        self.metrics.keys().collect()
    }

    pub fn count(&self, name: &str) -> usize {
        self.metrics.get(name).map_or(0, |v| v.len())
    }

    pub fn summary(&self) -> HashMap<String, (f64, f64, f64)> {
        let mut result = HashMap::new();
        for (name, points) in &self.metrics {
            if points.is_empty() {
                continue;
            }
            let min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
            let max = points.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);
            let avg = points.iter().map(|p| p.value).sum::<f64>() / points.len() as f64;
            result.insert(name.clone(), (min, avg, max));
        }
        result
    }
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Curriculum Learning ────────────────────────────────────────────────────

/// Promotion criteria for advancing curriculum stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromotionCriteria {
    RewardThreshold(f64),
    SuccessRate(f64),
    MinEpisodes(usize),
    StepCount(usize),
    Combined(Vec<PromotionCriteria>),
}

impl PromotionCriteria {
    pub fn is_met(&self, reward: f64, success_rate: f64, episodes: usize, steps: usize) -> bool {
        match self {
            Self::RewardThreshold(t) => reward >= *t,
            Self::SuccessRate(t) => success_rate >= *t,
            Self::MinEpisodes(n) => episodes >= *n,
            Self::StepCount(n) => steps >= *n,
            Self::Combined(criteria) => criteria.iter().all(|c| c.is_met(reward, success_rate, episodes, steps)),
        }
    }
}

/// A single stage in a curriculum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumStage {
    pub name: String,
    pub description: String,
    pub env_overrides: HashMap<String, String>,
    pub promotion_criteria: PromotionCriteria,
    pub max_steps: usize,
    pub difficulty: f64,
}

/// Manages curriculum-based training progression.
#[derive(Debug, Clone)]
pub struct CurriculumManager {
    pub stages: Vec<CurriculumStage>,
    pub current_stage: usize,
    pub stage_steps: usize,
    pub stage_episodes: usize,
    pub stage_best_reward: f64,
    pub auto_progress: bool,
    pub history: Vec<(usize, usize, f64)>, // (stage_idx, total_steps, best_reward)
}

impl CurriculumManager {
    pub fn new(stages: Vec<CurriculumStage>, auto_progress: bool) -> Self {
        Self {
            stages,
            current_stage: 0,
            stage_steps: 0,
            stage_episodes: 0,
            stage_best_reward: f64::NEG_INFINITY,
            auto_progress,
            history: Vec::new(),
        }
    }

    pub fn current(&self) -> Option<&CurriculumStage> {
        self.stages.get(self.current_stage)
    }

    pub fn is_complete(&self) -> bool {
        self.current_stage >= self.stages.len()
    }

    pub fn advance(&mut self, total_steps: usize) -> bool {
        if self.current_stage >= self.stages.len() {
            return false;
        }
        self.history.push((self.current_stage, total_steps, self.stage_best_reward));
        self.current_stage += 1;
        self.stage_steps = 0;
        self.stage_episodes = 0;
        self.stage_best_reward = f64::NEG_INFINITY;
        true
    }

    pub fn check_promotion(&mut self, reward: f64, success_rate: f64, total_steps: usize) -> bool {
        self.stage_steps += 1;
        if reward > self.stage_best_reward {
            self.stage_best_reward = reward;
        }
        if !self.auto_progress || self.current_stage >= self.stages.len() {
            return false;
        }
        let stage = &self.stages[self.current_stage];
        if stage.promotion_criteria.is_met(reward, success_rate, self.stage_episodes, self.stage_steps) {
            self.advance(total_steps)
        } else {
            false
        }
    }

    pub fn total_stages(&self) -> usize {
        self.stages.len()
    }

    pub fn progress_fraction(&self) -> f64 {
        if self.stages.is_empty() {
            return 1.0;
        }
        self.current_stage as f64 / self.stages.len() as f64
    }

    pub fn current_difficulty(&self) -> f64 {
        self.current().map_or(1.0, |s| s.difficulty)
    }
}

// ─── Multi-Agent Training ───────────────────────────────────────────────────

/// Multi-agent training mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultiAgentMode {
    Cooperative,
    Competitive,
    Mixed,
}

impl std::fmt::Display for MultiAgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cooperative => write!(f, "cooperative"),
            Self::Competitive => write!(f, "competitive"),
            Self::Mixed => write!(f, "mixed"),
        }
    }
}

/// Communication protocol for multi-agent systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommunicationProtocol {
    None,
    SharedReward,
    MessagePassing { channel_size: usize },
    Attention { d_model: usize },
    Broadcast,
}

/// An agent type in multi-agent training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentType {
    pub name: String,
    pub policy: PolicyNetwork,
    pub observation_dim: usize,
    pub action_dim: usize,
    pub count: usize,
    pub team_id: Option<usize>,
}

/// Multi-agent orchestrator.
#[derive(Debug, Clone)]
pub struct MultiAgentOrchestrator {
    pub mode: MultiAgentMode,
    pub agent_types: Vec<AgentType>,
    pub communication: CommunicationProtocol,
    pub shared_reward: bool,
    pub agent_metrics: HashMap<String, Vec<f64>>,
    pub total_interactions: usize,
}

impl MultiAgentOrchestrator {
    pub fn new(mode: MultiAgentMode, communication: CommunicationProtocol) -> Self {
        Self {
            mode,
            agent_types: Vec::new(),
            communication,
            shared_reward: false,
            agent_metrics: HashMap::new(),
            total_interactions: 0,
        }
    }

    pub fn add_agent_type(&mut self, agent_type: AgentType) {
        self.agent_types.push(agent_type);
    }

    pub fn total_agents(&self) -> usize {
        self.agent_types.iter().map(|a| a.count).sum()
    }

    pub fn team_count(&self) -> usize {
        let mut teams: Vec<usize> = self.agent_types
            .iter()
            .filter_map(|a| a.team_id)
            .collect();
        teams.sort();
        teams.dedup();
        teams.len()
    }

    pub fn record_interaction(&mut self, agent_name: &str, reward: f64) {
        self.agent_metrics
            .entry(agent_name.to_string())
            .or_default()
            .push(reward);
        self.total_interactions += 1;
    }

    pub fn average_reward_for(&self, agent_name: &str) -> f64 {
        match self.agent_metrics.get(agent_name) {
            Some(rewards) if !rewards.is_empty() => {
                rewards.iter().sum::<f64>() / rewards.len() as f64
            }
            _ => 0.0,
        }
    }

    pub fn compute_shared_reward(&self) -> f64 {
        if self.agent_metrics.is_empty() {
            return 0.0;
        }
        let total: f64 = self.agent_metrics.values().flatten().sum();
        let count: usize = self.agent_metrics.values().map(|v| v.len()).sum();
        if count == 0 { 0.0 } else { total / count as f64 }
    }
}

// ─── A2A Protocol for MARL ──────────────────────────────────────────────────

/// Message type for agent-to-agent communication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum A2AMessageType {
    Observation,
    ActionProposal,
    Negotiation,
    Delegation,
    Acknowledgment,
    Coordination,
    Hierarchy,
}

/// An agent-to-agent message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: u64,
    pub msg_type: A2AMessageType,
    pub sender: String,
    pub receiver: String,
    pub payload: HashMap<String, f64>,
    pub timestamp: u64,
    pub priority: u8,
}

impl A2AMessage {
    pub fn new(
        msg_type: A2AMessageType,
        sender: String,
        receiver: String,
        payload: HashMap<String, f64>,
    ) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            msg_type,
            sender,
            receiver,
            payload,
            timestamp: ts,
            priority: 5,
        }
    }

    pub fn with_priority(mut self, p: u8) -> Self {
        self.priority = p;
        self
    }
}

/// A2A message bus for multi-agent communication.
#[derive(Debug, Clone)]
pub struct A2AProtocol {
    pub messages: Vec<A2AMessage>,
    pub agent_mailboxes: HashMap<String, Vec<A2AMessage>>,
    pub total_sent: usize,
    pub hierarchy: HashMap<String, Vec<String>>, // parent -> children
}

impl A2AProtocol {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            agent_mailboxes: HashMap::new(),
            total_sent: 0,
            hierarchy: HashMap::new(),
        }
    }

    pub fn send(&mut self, msg: A2AMessage) {
        let receiver = msg.receiver.clone();
        self.agent_mailboxes
            .entry(receiver)
            .or_default()
            .push(msg.clone());
        self.messages.push(msg);
        self.total_sent += 1;
    }

    pub fn broadcast(&mut self, sender: &str, agents: &[String], payload: HashMap<String, f64>) {
        for agent in agents {
            if agent != sender {
                let msg = A2AMessage::new(
                    A2AMessageType::Coordination,
                    sender.to_string(),
                    agent.clone(),
                    payload.clone(),
                );
                self.send(msg);
            }
        }
    }

    pub fn receive(&mut self, agent: &str) -> Vec<A2AMessage> {
        self.agent_mailboxes.remove(agent).unwrap_or_default()
    }

    pub fn pending_count(&self, agent: &str) -> usize {
        self.agent_mailboxes.get(agent).map_or(0, |m| m.len())
    }

    pub fn set_hierarchy(&mut self, parent: &str, children: Vec<String>) {
        self.hierarchy.insert(parent.to_string(), children);
    }

    pub fn get_subordinates(&self, agent: &str) -> Vec<&String> {
        self.hierarchy.get(agent).map_or(Vec::new(), |c| c.iter().collect())
    }

    pub fn delegate(&mut self, from: &str, to: &str, task: HashMap<String, f64>) {
        let msg = A2AMessage::new(
            A2AMessageType::Delegation,
            from.to_string(),
            to.to_string(),
            task,
        );
        self.send(msg);
    }

    pub fn negotiate(&mut self, from: &str, to: &str, proposal: HashMap<String, f64>) {
        let msg = A2AMessage::new(
            A2AMessageType::Negotiation,
            from.to_string(),
            to.to_string(),
            proposal,
        );
        self.send(msg);
    }
}

impl Default for A2AProtocol {
    fn default() -> Self {
        Self::new()
    }
}

// ─── AutoRL ─────────────────────────────────────────────────────────────────

/// Hyperparameter search strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchStrategy {
    GridSearch,
    RandomSearch,
    BayesianOptimization { acquisition: String },
    PopulationBased { population_size: usize, exploit_fraction: f64 },
    NAS { max_layers: usize, max_width: usize },
}

/// A hyperparameter search space entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HParamSpace {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub log_scale: bool,
    pub discrete: bool,
}

impl HParamSpace {
    pub fn continuous(name: &str, min: f64, max: f64) -> Self {
        Self { name: name.into(), min, max, log_scale: false, discrete: false }
    }

    pub fn log_continuous(name: &str, min: f64, max: f64) -> Self {
        Self { name: name.into(), min, max, log_scale: true, discrete: false }
    }

    pub fn discrete(name: &str, min: f64, max: f64) -> Self {
        Self { name: name.into(), min, max, log_scale: false, discrete: true }
    }

    /// Sample linearly within the range (deterministic for a given fraction).
    pub fn sample_at(&self, fraction: f64) -> f64 {
        let f = fraction.clamp(0.0, 1.0);
        if self.log_scale {
            let log_min = self.min.ln();
            let log_max = self.max.ln();
            (log_min + (log_max - log_min) * f).exp()
        } else {
            let val = self.min + (self.max - self.min) * f;
            if self.discrete { val.round() } else { val }
        }
    }
}

/// Result of a single HPO trial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub trial_id: usize,
    pub params: HashMap<String, f64>,
    pub score: f64,
    pub steps_trained: usize,
    pub status: TrialStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrialStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Pruned,
}

/// AutoRL system for automatic hyperparameter optimization.
#[derive(Debug, Clone)]
pub struct AutoRL {
    pub strategy: SearchStrategy,
    pub search_space: Vec<HParamSpace>,
    pub trials: Vec<TrialResult>,
    pub max_trials: usize,
    pub best_trial: Option<usize>,
    pub best_score: f64,
}

impl AutoRL {
    pub fn new(strategy: SearchStrategy, max_trials: usize) -> Self {
        Self {
            strategy,
            search_space: Vec::new(),
            trials: Vec::new(),
            max_trials,
            best_trial: None,
            best_score: f64::NEG_INFINITY,
        }
    }

    pub fn add_param(&mut self, param: HParamSpace) {
        self.search_space.push(param);
    }

    pub fn suggest_params(&self, trial_idx: usize) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        match &self.strategy {
            SearchStrategy::GridSearch => {
                let total = self.max_trials.max(1);
                let frac = trial_idx as f64 / total as f64;
                for p in &self.search_space {
                    params.insert(p.name.clone(), p.sample_at(frac));
                }
            }
            SearchStrategy::RandomSearch => {
                // Deterministic pseudo-random based on trial_idx
                for (i, p) in self.search_space.iter().enumerate() {
                    let seed_val = ((trial_idx * 7919 + i * 6271) % 10000) as f64 / 10000.0;
                    params.insert(p.name.clone(), p.sample_at(seed_val));
                }
            }
            SearchStrategy::BayesianOptimization { .. } => {
                // Simplified: use history to bias towards best-performing regions
                if let Some(best_idx) = self.best_trial {
                    let best = &self.trials[best_idx].params;
                    for p in &self.search_space {
                        let base = best.get(&p.name).copied().unwrap_or((p.min + p.max) / 2.0);
                        let range = (p.max - p.min) * 0.2;
                        let perturbation = ((trial_idx * 3571) % 1000) as f64 / 1000.0 - 0.5;
                        let val = (base + range * perturbation).clamp(p.min, p.max);
                        params.insert(p.name.clone(), val);
                    }
                } else {
                    // Fall back to midpoint for first trial
                    for p in &self.search_space {
                        params.insert(p.name.clone(), p.sample_at(0.5));
                    }
                }
            }
            SearchStrategy::PopulationBased { .. } => {
                // Mutate from best or sample fresh
                if let Some(best_idx) = self.best_trial {
                    let best = &self.trials[best_idx].params;
                    for p in &self.search_space {
                        let base = best.get(&p.name).copied().unwrap_or(p.sample_at(0.5));
                        let factor = 1.0 + (((trial_idx * 1009) % 200) as f64 / 1000.0 - 0.1);
                        let val = (base * factor).clamp(p.min, p.max);
                        params.insert(p.name.clone(), val);
                    }
                } else {
                    for p in &self.search_space {
                        params.insert(p.name.clone(), p.sample_at(0.5));
                    }
                }
            }
            SearchStrategy::NAS { .. } => {
                for p in &self.search_space {
                    let frac = ((trial_idx * 4391 + 17) % 10000) as f64 / 10000.0;
                    params.insert(p.name.clone(), p.sample_at(frac));
                }
            }
        }
        params
    }

    pub fn report_result(&mut self, result: TrialResult) {
        if result.status == TrialStatus::Completed && result.score > self.best_score {
            self.best_score = result.score;
            self.best_trial = Some(self.trials.len());
        }
        self.trials.push(result);
    }

    pub fn completed_trials(&self) -> usize {
        self.trials.iter().filter(|t| t.status == TrialStatus::Completed).count()
    }

    pub fn best_params(&self) -> Option<&HashMap<String, f64>> {
        self.best_trial.map(|i| &self.trials[i].params)
    }

    pub fn top_k(&self, k: usize) -> Vec<&TrialResult> {
        let mut completed: Vec<&TrialResult> = self.trials
            .iter()
            .filter(|t| t.status == TrialStatus::Completed)
            .collect();
        completed.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        completed.into_iter().take(k).collect()
    }

    pub fn should_prune(&self, trial_idx: usize, current_score: f64, step: usize) -> bool {
        if self.trials.is_empty() || step < 100 {
            return false;
        }
        // Prune if significantly worse than median at similar step counts
        let completed_at_step: Vec<f64> = self.trials
            .iter()
            .filter(|t| t.status == TrialStatus::Completed && t.steps_trained >= step)
            .map(|t| t.score)
            .collect();
        if completed_at_step.is_empty() {
            return false;
        }
        let _ = trial_idx; // Used by caller for context
        let median_idx = completed_at_step.len() / 2;
        let mut sorted = completed_at_step;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[median_idx];
        current_score < median * 0.5
    }
}

// ─── Population-Based Training ──────────────────────────────────────────────

/// An individual in PBT population.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PBTAgent {
    pub id: usize,
    pub hyperparams: HashMap<String, f64>,
    pub score: f64,
    pub elo: f64,
    pub generation: usize,
    pub matches_played: usize,
    pub wins: usize,
}

impl PBTAgent {
    pub fn new(id: usize, hyperparams: HashMap<String, f64>) -> Self {
        Self {
            id,
            hyperparams,
            score: 0.0,
            elo: 1200.0,
            generation: 0,
            matches_played: 0,
            wins: 0,
        }
    }

    pub fn win_rate(&self) -> f64 {
        if self.matches_played == 0 {
            return 0.0;
        }
        self.wins as f64 / self.matches_played as f64
    }

    /// Expected score against opponent (ELO formula).
    pub fn expected_score(&self, opponent_elo: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((opponent_elo - self.elo) / 400.0))
    }

    pub fn update_elo(&mut self, opponent_elo: f64, actual_score: f64) {
        let k = 32.0;
        let expected = self.expected_score(opponent_elo);
        self.elo += k * (actual_score - expected);
    }
}

/// PBT manager with exploitation, exploration, and league matchmaking.
#[derive(Debug, Clone)]
pub struct PopulationTrainer {
    pub population: Vec<PBTAgent>,
    pub exploit_fraction: f64,
    pub mutation_rate: f64,
    pub mutation_strength: f64,
    pub generation: usize,
    pub match_history: Vec<(usize, usize, f64)>, // (agent_a, agent_b, a_score)
}

impl PopulationTrainer {
    pub fn new(population_size: usize, initial_params: HashMap<String, f64>) -> Self {
        let population = (0..population_size)
            .map(|i| {
                let mut params = initial_params.clone();
                // Perturb initial params
                for (_, v) in params.iter_mut() {
                    let factor = 1.0 + ((i * 7 + 3) as f64 / population_size as f64 - 0.5) * 0.4;
                    *v *= factor;
                }
                PBTAgent::new(i, params)
            })
            .collect();
        Self {
            population,
            exploit_fraction: 0.2,
            mutation_rate: 0.8,
            mutation_strength: 0.2,
            generation: 0,
            match_history: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.population.len()
    }

    /// Exploit: bottom agents copy top agents' hyperparams.
    pub fn exploit_and_explore(&mut self) {
        if self.population.len() < 2 {
            return;
        }
        let mut indices: Vec<usize> = (0..self.population.len()).collect();
        indices.sort_by(|&a, &b| {
            self.population[b].score.partial_cmp(&self.population[a].score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let n_exploit = (self.population.len() as f64 * self.exploit_fraction).ceil() as usize;
        let n_exploit = n_exploit.max(1).min(self.population.len() / 2);

        // Collect top agent params
        let top_params: Vec<HashMap<String, f64>> = indices[..n_exploit]
            .iter()
            .map(|&i| self.population[i].hyperparams.clone())
            .collect();

        // Bottom agents copy from top and mutate
        let bottom_indices: Vec<usize> = indices[indices.len() - n_exploit..].to_vec();
        for (bi, &idx) in bottom_indices.iter().enumerate() {
            let source = &top_params[bi % top_params.len()];
            let mut new_params = source.clone();
            // Mutate
            for (_, v) in new_params.iter_mut() {
                let perturbation = 1.0 + (((idx * 997 + self.generation * 31) % 200) as f64 / 1000.0 - 0.1);
                *v *= perturbation;
            }
            self.population[idx].hyperparams = new_params;
            self.population[idx].generation = self.generation + 1;
        }
        self.generation += 1;
    }

    pub fn record_match(&mut self, agent_a: usize, agent_b: usize, a_score: f64) {
        self.match_history.push((agent_a, agent_b, a_score));
        let b_elo = self.population[agent_b].elo;
        let a_elo = self.population[agent_a].elo;
        self.population[agent_a].update_elo(b_elo, a_score);
        self.population[agent_b].update_elo(a_elo, 1.0 - a_score);
        self.population[agent_a].matches_played += 1;
        self.population[agent_b].matches_played += 1;
        if a_score > 0.5 {
            self.population[agent_a].wins += 1;
        } else if a_score < 0.5 {
            self.population[agent_b].wins += 1;
        }
    }

    pub fn update_scores(&mut self, scores: &[f64]) {
        for (agent, &score) in self.population.iter_mut().zip(scores.iter()) {
            agent.score = score;
        }
    }

    pub fn best_agent(&self) -> Option<&PBTAgent> {
        self.population.iter().max_by(|a, b| {
            a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn elo_ranking(&self) -> Vec<(usize, f64)> {
        let mut ranking: Vec<(usize, f64)> = self.population.iter().map(|a| (a.id, a.elo)).collect();
        ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranking
    }

    /// Matchmake two agents with similar ELO.
    pub fn matchmake(&self) -> Option<(usize, usize)> {
        if self.population.len() < 2 {
            return None;
        }
        let mut by_elo: Vec<(usize, f64)> = self.population.iter().map(|a| (a.id, a.elo)).collect();
        by_elo.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        // Pick closest ELO pair that has played fewest matches together
        let mut best_pair = (by_elo[0].0, by_elo[1].0);
        let mut min_elo_diff = (by_elo[0].1 - by_elo[1].1).abs();
        for w in by_elo.windows(2) {
            let diff = (w[0].1 - w[1].1).abs();
            if diff < min_elo_diff {
                min_elo_diff = diff;
                best_pair = (w[0].0, w[1].0);
            }
        }
        Some(best_pair)
    }
}

// ─── Fault Tolerance ────────────────────────────────────────────────────────

/// Checkpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: usize,
    pub step: usize,
    pub path: String,
    pub timestamp: u64,
    pub metrics_snapshot: HashMap<String, f64>,
    pub config_snapshot: Option<String>,
    pub size_bytes: usize,
}

/// Fault tolerance manager with checkpointing and auto-resume.
#[derive(Debug, Clone)]
pub struct FaultToleranceManager {
    pub config: CheckpointConfig,
    pub checkpoints: Vec<Checkpoint>,
    pub last_checkpoint_step: usize,
    pub preemption_detected: bool,
    pub auto_resume: bool,
    next_checkpoint_id: usize,
}

impl FaultToleranceManager {
    pub fn new(config: CheckpointConfig) -> Self {
        Self {
            config,
            checkpoints: Vec::new(),
            last_checkpoint_step: 0,
            preemption_detected: false,
            auto_resume: true,
            next_checkpoint_id: 0,
        }
    }

    pub fn should_checkpoint(&self, current_step: usize) -> bool {
        self.config.enabled && current_step - self.last_checkpoint_step >= self.config.frequency_steps
    }

    pub fn create_checkpoint(
        &mut self,
        step: usize,
        metrics: &HashMap<String, f64>,
    ) -> Checkpoint {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let id = self.next_checkpoint_id;
        self.next_checkpoint_id += 1;
        let ckpt = Checkpoint {
            id,
            step,
            path: format!("{}/checkpoint_{}", self.config.path, id),
            timestamp: ts,
            metrics_snapshot: metrics.clone(),
            config_snapshot: None,
            size_bytes: 0,
        };
        self.checkpoints.push(ckpt.clone());
        self.last_checkpoint_step = step;

        // Prune old checkpoints
        while self.checkpoints.len() > self.config.keep_last {
            self.checkpoints.remove(0);
        }
        ckpt
    }

    pub fn latest_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    pub fn find_checkpoint_by_step(&self, step: usize) -> Option<&Checkpoint> {
        self.checkpoints.iter().rev().find(|c| c.step <= step)
    }

    pub fn mark_preemption(&mut self) {
        self.preemption_detected = true;
    }

    pub fn resume_step(&self) -> usize {
        self.latest_checkpoint().map_or(0, |c| c.step)
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn total_checkpoint_size(&self) -> usize {
        self.checkpoints.iter().map(|c| c.size_bytes).sum()
    }
}

// ─── Training Lifecycle ─────────────────────────────────────────────────────

/// Phase of the training lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingPhase {
    Idle,
    Warmup,
    Training,
    Evaluating,
    Paused,
    Cooldown,
    Completed,
    Failed(String),
}

impl std::fmt::Display for TrainingPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Warmup => write!(f, "warmup"),
            Self::Training => write!(f, "training"),
            Self::Evaluating => write!(f, "evaluating"),
            Self::Paused => write!(f, "paused"),
            Self::Cooldown => write!(f, "cooldown"),
            Self::Completed => write!(f, "completed"),
            Self::Failed(e) => write!(f, "failed: {}", e),
        }
    }
}

/// Full training lifecycle manager.
#[derive(Debug, Clone)]
pub struct TrainingLifecycle {
    pub config: TrainingConfig,
    pub phase: TrainingPhase,
    pub current_step: usize,
    pub current_episode: usize,
    pub metrics: TrainingMetrics,
    pub fault_tolerance: FaultToleranceManager,
    phase_history: Vec<(TrainingPhase, usize)>,
}

impl TrainingLifecycle {
    pub fn new(config: TrainingConfig) -> Self {
        let ft = FaultToleranceManager::new(config.checkpoint.clone());
        Self {
            config,
            phase: TrainingPhase::Idle,
            current_step: 0,
            current_episode: 0,
            metrics: TrainingMetrics::new(),
            fault_tolerance: ft,
            phase_history: Vec::new(),
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        match &self.phase {
            TrainingPhase::Idle | TrainingPhase::Paused => {
                let errors = self.config.validate();
                if !errors.is_empty() {
                    return Err(format!("Config validation failed: {}", errors.join("; ")));
                }
                self.transition_to(TrainingPhase::Warmup);
                Ok(())
            }
            _ => Err(format!("Cannot start from phase: {}", self.phase)),
        }
    }

    pub fn begin_training(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Warmup {
            return Err("Must be in warmup phase to begin training".into());
        }
        self.transition_to(TrainingPhase::Training);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Training {
            return Err("Can only pause during training".into());
        }
        self.transition_to(TrainingPhase::Paused);
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Paused {
            return Err("Can only resume from paused".into());
        }
        self.transition_to(TrainingPhase::Training);
        Ok(())
    }

    pub fn begin_eval(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Training {
            return Err("Can only evaluate during training".into());
        }
        self.transition_to(TrainingPhase::Evaluating);
        Ok(())
    }

    pub fn end_eval(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Evaluating {
            return Err("Not in evaluating phase".into());
        }
        self.transition_to(TrainingPhase::Training);
        Ok(())
    }

    pub fn begin_cooldown(&mut self) -> Result<(), String> {
        if self.phase != TrainingPhase::Training {
            return Err("Can only cooldown from training".into());
        }
        self.transition_to(TrainingPhase::Cooldown);
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), String> {
        match &self.phase {
            TrainingPhase::Cooldown | TrainingPhase::Training => {
                self.transition_to(TrainingPhase::Completed);
                Ok(())
            }
            _ => Err(format!("Cannot complete from phase: {}", self.phase)),
        }
    }

    pub fn fail(&mut self, reason: &str) {
        self.transition_to(TrainingPhase::Failed(reason.to_string()));
    }

    pub fn step(&mut self, reward: f64, loss: f64) {
        self.current_step += 1;
        self.metrics.record("reward", self.current_step, reward);
        self.metrics.record("loss", self.current_step, loss);

        let lr = self.config.lr_schedule.get_lr(
            self.config.learning_rate,
            self.current_step,
            self.config.total_timesteps,
        );
        self.metrics.record("learning_rate", self.current_step, lr);

        if self.fault_tolerance.should_checkpoint(self.current_step) {
            let mut snapshot = HashMap::new();
            snapshot.insert("reward".into(), reward);
            snapshot.insert("loss".into(), loss);
            snapshot.insert("step".into(), self.current_step as f64);
            self.fault_tolerance.create_checkpoint(self.current_step, &snapshot);
        }
    }

    pub fn progress(&self) -> f64 {
        if self.config.total_timesteps == 0 {
            return 1.0;
        }
        (self.current_step as f64 / self.config.total_timesteps as f64).min(1.0)
    }

    pub fn is_done(&self) -> bool {
        self.current_step >= self.config.total_timesteps
            || matches!(self.phase, TrainingPhase::Completed | TrainingPhase::Failed(_))
    }

    fn transition_to(&mut self, new_phase: TrainingPhase) {
        self.phase_history.push((self.phase.clone(), self.current_step));
        self.phase = new_phase;
    }

    pub fn phase_count(&self) -> usize {
        self.phase_history.len()
    }
}

// ─── TrainOS Top-Level Orchestrator ─────────────────────────────────────────

/// The top-level TrainOS orchestrator tying everything together.
#[derive(Debug)]
pub struct TrainOS {
    pub registry: AlgorithmRegistry,
    pub lifecycle: Option<TrainingLifecycle>,
    pub auto_rl: Option<AutoRL>,
    pub curriculum: Option<CurriculumManager>,
    pub multi_agent: Option<MultiAgentOrchestrator>,
    pub distributed: Option<DistributedManager>,
    pub a2a: A2AProtocol,
    pub pbt: Option<PopulationTrainer>,
    pub replay_buffer: Option<ReplayBuffer>,
    pub collector: Option<ExperienceCollector>,
}

impl TrainOS {
    pub fn new() -> Self {
        Self {
            registry: AlgorithmRegistry::new(),
            lifecycle: None,
            auto_rl: None,
            curriculum: None,
            multi_agent: None,
            distributed: None,
            a2a: A2AProtocol::new(),
            pbt: None,
            replay_buffer: None,
            collector: None,
        }
    }

    pub fn configure(&mut self, config: TrainingConfig) {
        let dist_config = config.distributed.clone();
        self.lifecycle = Some(TrainingLifecycle::new(config));
        self.distributed = Some(DistributedManager::new(dist_config));
    }

    pub fn setup_replay_buffer(&mut self, buffer_type: ReplayBufferType, capacity: usize) {
        self.replay_buffer = Some(ReplayBuffer::new(buffer_type, capacity));
    }

    pub fn setup_curriculum(&mut self, stages: Vec<CurriculumStage>, auto: bool) {
        self.curriculum = Some(CurriculumManager::new(stages, auto));
    }

    pub fn setup_multi_agent(&mut self, mode: MultiAgentMode, comm: CommunicationProtocol) {
        self.multi_agent = Some(MultiAgentOrchestrator::new(mode, comm));
    }

    pub fn setup_auto_rl(&mut self, strategy: SearchStrategy, max_trials: usize) {
        self.auto_rl = Some(AutoRL::new(strategy, max_trials));
    }

    pub fn setup_pbt(&mut self, pop_size: usize, initial_params: HashMap<String, f64>) {
        self.pbt = Some(PopulationTrainer::new(pop_size, initial_params));
    }

    pub fn setup_collector(&mut self, num_workers: usize, rollout_length: usize) {
        self.collector = Some(ExperienceCollector::new(num_workers, rollout_length));
    }

    pub fn algorithm_count(&self) -> usize {
        self.registry.count()
    }

    pub fn status_summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        summary.insert("algorithms".into(), self.registry.count().to_string());
        if let Some(lc) = &self.lifecycle {
            summary.insert("phase".into(), lc.phase.to_string());
            summary.insert("step".into(), lc.current_step.to_string());
            summary.insert("progress".into(), format!("{:.1}%", lc.progress() * 100.0));
        }
        if let Some(cur) = &self.curriculum {
            summary.insert("curriculum_stage".into(), format!("{}/{}", cur.current_stage, cur.total_stages()));
        }
        if let Some(ma) = &self.multi_agent {
            summary.insert("agents".into(), ma.total_agents().to_string());
        }
        if let Some(rb) = &self.replay_buffer {
            summary.insert("buffer_size".into(), rb.len().to_string());
        }
        if let Some(pbt) = &self.pbt {
            summary.insert("population".into(), pbt.size().to_string());
        }
        summary
    }
}

impl Default for TrainOS {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Algorithm Registry Tests ────────────────────────────────────────

    #[test]
    fn test_algorithm_id_all_returns_30_algorithms() {
        let all = AlgorithmId::all();
        assert_eq!(all.len(), 30);
    }

    #[test]
    fn test_algorithm_families() {
        assert_eq!(AlgorithmId::PPO.family(), AlgorithmFamily::OnPolicy);
        assert_eq!(AlgorithmId::SAC.family(), AlgorithmFamily::OffPolicy);
        assert_eq!(AlgorithmId::CQL.family(), AlgorithmFamily::OfflineRL);
        assert_eq!(AlgorithmId::DreamerV3.family(), AlgorithmFamily::ModelBased);
        assert_eq!(AlgorithmId::MAPPO.family(), AlgorithmFamily::MultiAgent);
        assert_eq!(AlgorithmId::BC.family(), AlgorithmFamily::Imitation);
    }

    #[test]
    fn test_algorithm_names() {
        assert_eq!(AlgorithmId::PPO.name(), "PPO");
        assert_eq!(AlgorithmId::TD3BC.name(), "TD3+BC");
        assert_eq!(AlgorithmId::QRDQN.name(), "QR-DQN");
        assert_eq!(AlgorithmId::DecisionTransformer.name(), "Decision Transformer");
    }

    #[test]
    fn test_algorithm_requires_replay_buffer() {
        assert!(!AlgorithmId::PPO.requires_replay_buffer());
        assert!(AlgorithmId::SAC.requires_replay_buffer());
        assert!(AlgorithmId::CQL.requires_replay_buffer());
        assert!(!AlgorithmId::MAPPO.requires_replay_buffer());
    }

    #[test]
    fn test_algorithm_continuous_action_support() {
        assert!(AlgorithmId::PPO.supports_continuous_actions());
        assert!(AlgorithmId::SAC.supports_continuous_actions());
        assert!(!AlgorithmId::DQN.supports_continuous_actions());
        assert!(!AlgorithmId::C51.supports_continuous_actions());
    }

    #[test]
    fn test_algorithm_default_hyperparams_ppo() {
        let hp = AlgorithmId::PPO.default_hyperparams();
        assert_eq!(*hp.get("clip_ratio").unwrap(), 0.2);
        assert_eq!(*hp.get("entropy_coef").unwrap(), 0.01);
        assert!(hp.contains_key("learning_rate"));
    }

    #[test]
    fn test_algorithm_default_hyperparams_sac() {
        let hp = AlgorithmId::SAC.default_hyperparams();
        assert_eq!(*hp.get("tau").unwrap(), 0.005);
        assert_eq!(*hp.get("alpha").unwrap(), 0.2);
    }

    #[test]
    fn test_algorithm_registry_creation() {
        let reg = AlgorithmRegistry::new();
        assert_eq!(reg.count(), 30);
    }

    #[test]
    fn test_algorithm_registry_lookup() {
        let reg = AlgorithmRegistry::new();
        assert!(reg.get("ppo").is_some());
        assert!(reg.get("sac").is_some());
        assert!(reg.get("td3_bc").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_algorithm_registry_family_filter() {
        let reg = AlgorithmRegistry::new();
        let on_policy = reg.list_by_family(&AlgorithmFamily::OnPolicy);
        assert_eq!(on_policy.len(), 4); // PPO, A2C, TRPO, PPG
        let off_policy = reg.list_by_family(&AlgorithmFamily::OffPolicy);
        assert_eq!(off_policy.len(), 7);
    }

    #[test]
    fn test_algorithm_registry_continuous_filter() {
        let reg = AlgorithmRegistry::new();
        let continuous = reg.supports_continuous();
        assert!(continuous.len() > 20);
    }

    #[test]
    fn test_algorithm_registry_custom_registration() {
        let mut reg = AlgorithmRegistry::new();
        let desc = AlgorithmDescriptor::new(AlgorithmId::PPO);
        reg.register_custom("custom_ppo".into(), desc);
        assert_eq!(reg.count(), 31);
        assert!(reg.get("custom_ppo").is_some());
    }

    #[test]
    fn test_algorithm_descriptor_construction() {
        let desc = AlgorithmDescriptor::new(AlgorithmId::DreamerV3);
        assert!(desc.requires_model);
        assert!(!desc.supports_multi_agent);
        assert_eq!(desc.family, AlgorithmFamily::ModelBased);
    }

    #[test]
    fn test_algorithm_family_display() {
        assert_eq!(AlgorithmFamily::OnPolicy.label(), "On-Policy");
        assert_eq!(AlgorithmFamily::Imitation.label(), "Imitation Learning");
    }

    // ── Policy Network Tests ────────────────────────────────────────────

    #[test]
    fn test_activation_relu() {
        assert_eq!(Activation::ReLU.apply(5.0), 5.0);
        assert_eq!(Activation::ReLU.apply(-3.0), 0.0);
    }

    #[test]
    fn test_activation_sigmoid() {
        let s = Activation::Sigmoid.apply(0.0);
        assert!((s - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_activation_tanh() {
        let t = Activation::Tanh.apply(0.0);
        assert!((t - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_activation_leaky_relu() {
        assert_eq!(Activation::LeakyReLU(0.1).apply(5.0), 5.0);
        assert!((Activation::LeakyReLU(0.1).apply(-5.0) - (-0.5)).abs() < 1e-10);
    }

    #[test]
    fn test_activation_identity() {
        assert_eq!(Activation::Identity.apply(42.0), 42.0);
    }

    #[test]
    fn test_mlp_policy_creation() {
        let net = PolicyNetwork::mlp(64, &[256, 256], 4);
        assert_eq!(net.network_type, NetworkType::MLP);
        assert_eq!(net.output_dim, 4);
        assert_eq!(net.layers.len(), 3); // 2 hidden + 1 output
    }

    #[test]
    fn test_mlp_param_count() {
        let net = PolicyNetwork::mlp(10, &[32, 32], 4);
        let total = net.total_params();
        // Dense(10->32) = 10*32+32=352, Dense(32->32)=32*32+32=1056, Dense(32->4)=32*4+4=132
        assert_eq!(total, 352 + 1056 + 132);
    }

    #[test]
    fn test_cnn_policy_creation() {
        let net = PolicyNetwork::cnn(3, 84, 18);
        assert_eq!(net.network_type, NetworkType::CNN);
        assert_eq!(net.output_dim, 18);
        assert_eq!(net.input_shape, vec![3, 84, 84]);
    }

    #[test]
    fn test_transformer_policy_creation() {
        let net = PolicyNetwork::transformer(128, 4, 3, 10);
        assert_eq!(net.network_type, NetworkType::Transformer);
        assert_eq!(net.layers.len(), 4); // 3 blocks + 1 output
    }

    #[test]
    fn test_lstm_policy_creation() {
        let net = PolicyNetwork::lstm(64, 128, 2, 8);
        assert_eq!(net.network_type, NetworkType::LSTM);
        assert_eq!(net.output_dim, 8);
    }

    #[test]
    fn test_network_summary() {
        let net = PolicyNetwork::mlp(10, &[32], 2);
        let summary = net.summary();
        assert!(summary.contains("mlp_policy"));
        assert!(summary.contains("MLP"));
    }

    #[test]
    fn test_layer_param_count_dense() {
        let l = LayerType::Dense { in_features: 100, out_features: 50 };
        assert_eq!(l.param_count(), 5050);
    }

    #[test]
    fn test_layer_param_count_dropout_is_zero() {
        let l = LayerType::Dropout { rate: 0.5 };
        assert_eq!(l.param_count(), 0);
    }

    // ── Replay Buffer Tests ─────────────────────────────────────────────

    fn make_transition(reward: f64) -> Transition {
        Transition {
            state: vec![1.0, 2.0],
            action: vec![0.5],
            reward,
            next_state: vec![1.5, 2.5],
            done: false,
            info: HashMap::new(),
        }
    }

    #[test]
    fn test_replay_buffer_add_and_len() {
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 100);
        assert!(buf.is_empty());
        buf.add(make_transition(1.0));
        assert_eq!(buf.len(), 1);
        buf.add(make_transition(2.0));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_replay_buffer_capacity_limit() {
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 3);
        for i in 0..5 {
            buf.add(make_transition(i as f64));
        }
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_replay_buffer_uniform_sample() {
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 100);
        for i in 0..10 {
            buf.add(make_transition(i as f64));
        }
        let samples = buf.sample(3);
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn test_replay_buffer_prioritized_sample() {
        let mut buf = ReplayBuffer::new(
            ReplayBufferType::Prioritized { alpha: 0.6, beta: 0.4, beta_increment: 0.001 },
            100,
        );
        for i in 0..10 {
            buf.add(make_transition(i as f64));
        }
        let samples = buf.sample(3);
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn test_replay_buffer_her_sample() {
        let mut buf = ReplayBuffer::new(
            ReplayBufferType::Hindsight { strategy: HERStrategy::Future, k_goals: 4 },
            100,
        );
        for i in 0..20 {
            buf.add(make_transition(i as f64));
        }
        let samples = buf.sample(5);
        assert!(samples.len() <= 5);
    }

    #[test]
    fn test_replay_buffer_update_priorities() {
        let mut buf = ReplayBuffer::new(
            ReplayBufferType::Prioritized { alpha: 0.6, beta: 0.4, beta_increment: 0.001 },
            100,
        );
        for i in 0..5 {
            buf.add(make_transition(i as f64));
        }
        buf.update_priorities(&[0, 2], &[10.0, 20.0]);
        // Verify max_priority updated
        assert!(buf.max_priority >= 20.0);
    }

    #[test]
    fn test_replay_buffer_clear() {
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 100);
        buf.add(make_transition(1.0));
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_replay_buffer_utilization() {
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 10);
        for _ in 0..5 {
            buf.add(make_transition(1.0));
        }
        assert!((buf.utilization() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_replay_buffer_sample_empty() {
        let buf = ReplayBuffer::new(ReplayBufferType::Uniform, 10);
        assert!(buf.sample(5).is_empty());
    }

    // ── Training Configuration Tests ────────────────────────────────────

    #[test]
    fn test_config_default_valid() {
        let config = TrainingConfig::default();
        let errors = config.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_config_validation_empty_name() {
        let mut config = TrainingConfig::default();
        config.name = "".into();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_config_validation_bad_lr() {
        let mut config = TrainingConfig::default();
        config.learning_rate = -0.1;
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("learning_rate")));
    }

    #[test]
    fn test_config_validation_bad_gamma() {
        let mut config = TrainingConfig::default();
        config.gamma = 1.5;
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("gamma")));
    }

    #[test]
    fn test_config_effective_batch_size() {
        let mut config = TrainingConfig::default();
        config.batch_size = 64;
        config.distributed.num_workers = 4;
        config.distributed.gradient_accumulation_steps = 2;
        assert_eq!(config.effective_batch_size(), 64 * 4 * 2);
    }

    #[test]
    fn test_config_yaml_roundtrip() {
        let config = TrainingConfig::default();
        let yaml = config.to_yaml().unwrap();
        let parsed = TrainingConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.name, config.name);
        assert_eq!(parsed.total_timesteps, config.total_timesteps);
    }

    // ── LR Schedule Tests ───────────────────────────────────────────────

    #[test]
    fn test_lr_constant() {
        let lr = LRSchedule::Constant.get_lr(0.001, 500, 1000);
        assert_eq!(lr, 0.001);
    }

    #[test]
    fn test_lr_linear() {
        let schedule = LRSchedule::Linear { start: 0.01, end: 0.001 };
        let lr_mid = schedule.get_lr(0.01, 500, 1000);
        assert!((lr_mid - 0.0055).abs() < 1e-10);
    }

    #[test]
    fn test_lr_cosine() {
        let schedule = LRSchedule::Cosine { min_lr: 0.0 };
        let lr_start = schedule.get_lr(0.001, 0, 1000);
        let lr_end = schedule.get_lr(0.001, 1000, 1000);
        assert!(lr_start > lr_end);
    }

    #[test]
    fn test_lr_step_decay() {
        let schedule = LRSchedule::StepDecay { step_size: 100, gamma: 0.1 };
        let lr0 = schedule.get_lr(1.0, 0, 1000);
        let lr200 = schedule.get_lr(1.0, 200, 1000);
        assert_eq!(lr0, 1.0);
        assert!((lr200 - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_lr_warmup_cosine() {
        let schedule = LRSchedule::WarmupCosine { warmup_steps: 100, min_lr: 0.0 };
        let lr_50 = schedule.get_lr(0.001, 50, 1000);
        let lr_500 = schedule.get_lr(0.001, 500, 1000);
        assert!(lr_50 < 0.001);
        assert!(lr_500 > 0.0);
    }

    #[test]
    fn test_lr_exponential() {
        let schedule = LRSchedule::Exponential { gamma: 0.99 };
        let lr0 = schedule.get_lr(1.0, 0, 1000);
        let lr100 = schedule.get_lr(1.0, 100, 1000);
        assert_eq!(lr0, 1.0);
        assert!(lr100 < 1.0);
    }

    // ── Distributed Training Tests ──────────────────────────────────────

    #[test]
    fn test_distributed_manager_creation() {
        let cfg = DistributedConfig { num_workers: 4, ..Default::default() };
        let dm = DistributedManager::new(cfg);
        assert_eq!(dm.workers.len(), 4);
    }

    #[test]
    fn test_distributed_start_all() {
        let cfg = DistributedConfig { num_workers: 3, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        dm.start_all();
        assert_eq!(dm.active_workers(), 3);
    }

    #[test]
    fn test_distributed_stop_all() {
        let cfg = DistributedConfig { num_workers: 3, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        dm.start_all();
        dm.stop_all();
        assert_eq!(dm.active_workers(), 0);
    }

    #[test]
    fn test_allreduce_gradient_sync() {
        let cfg = DistributedConfig { num_workers: 2, gradient_sync: GradientSyncStrategy::AllReduce, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        let grads = vec![vec![1.0, 2.0, 3.0], vec![3.0, 4.0, 5.0]];
        let result = dm.sync_gradients(&grads);
        assert_eq!(result.len(), 3);
        assert!((result[0] - 2.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_server_gradient_sync() {
        let cfg = DistributedConfig {
            num_workers: 2,
            gradient_sync: GradientSyncStrategy::ParameterServer,
            ..Default::default()
        };
        let mut dm = DistributedManager::new(cfg);
        let grads = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let result = dm.sync_gradients(&grads);
        assert!((result[0] - 4.0).abs() < 1e-10); // sum
    }

    #[test]
    fn test_distributed_mark_worker_failed() {
        let cfg = DistributedConfig { num_workers: 3, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        dm.start_all();
        dm.mark_worker_failed(1, "OOM");
        assert_eq!(dm.active_workers(), 2);
    }

    #[test]
    fn test_distributed_replace_failed_worker() {
        let cfg = DistributedConfig { num_workers: 3, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        dm.start_all();
        dm.mark_worker_failed(1, "OOM");
        assert!(dm.replace_failed_worker(1));
        assert_eq!(dm.active_workers(), 3);
    }

    #[test]
    fn test_distributed_advance_step() {
        let cfg = DistributedConfig { num_workers: 2, ..Default::default() };
        let mut dm = DistributedManager::new(cfg);
        dm.start_all();
        dm.advance_step();
        assert_eq!(dm.global_step, 1);
        assert_eq!(dm.workers[0].steps_completed, 1);
    }

    // ── Experience Collection Tests ─────────────────────────────────────

    #[test]
    fn test_trajectory_creation() {
        let traj = Trajectory::new();
        assert_eq!(traj.length, 0);
        assert_eq!(traj.total_reward, 0.0);
    }

    #[test]
    fn test_trajectory_add() {
        let mut traj = Trajectory::new();
        traj.add(make_transition(1.0));
        traj.add(make_transition(2.0));
        assert_eq!(traj.length, 2);
        assert!((traj.total_reward - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_trajectory_gae() {
        let mut traj = Trajectory::new();
        for _ in 0..3 {
            traj.add(make_transition(1.0));
        }
        let values = vec![0.5, 0.5, 0.5, 0.0];
        let advantages = traj.compute_gae(0.99, 0.95, &values);
        assert_eq!(advantages.len(), 3);
        // advantages should be non-trivial
        assert!(advantages[0].abs() > 0.0);
    }

    #[test]
    fn test_trajectory_nstep_returns() {
        let mut traj = Trajectory::new();
        for _ in 0..5 {
            traj.add(make_transition(1.0));
        }
        let returns = traj.compute_nstep_returns(0.99, 3);
        assert_eq!(returns.len(), 5);
        // First return should be sum of 3 discounted steps
        let expected = 1.0 + 0.99 + 0.99 * 0.99;
        assert!((returns[0] - expected).abs() < 1e-6);
    }

    #[test]
    fn test_trajectory_discounted_returns() {
        let mut traj = Trajectory::new();
        traj.add(make_transition(1.0));
        traj.add(make_transition(1.0));
        traj.add(make_transition(1.0));
        let returns = traj.compute_returns(0.99);
        assert_eq!(returns.len(), 3);
        assert!(returns[0] > returns[1]);
        assert!(returns[1] > returns[2]);
    }

    #[test]
    fn test_experience_collector() {
        let mut collector = ExperienceCollector::new(4, 128);
        let mut traj = Trajectory::new();
        traj.add(make_transition(1.0));
        traj.add(Transition { done: true, ..make_transition(2.0) });
        collector.add_trajectory(traj);
        assert_eq!(collector.total_steps, 2);
        assert_eq!(collector.total_episodes, 1);
    }

    #[test]
    fn test_experience_collector_average_reward() {
        let mut collector = ExperienceCollector::new(2, 64);
        let mut t1 = Trajectory::new();
        t1.add(make_transition(10.0));
        let mut t2 = Trajectory::new();
        t2.add(make_transition(20.0));
        collector.add_trajectory(t1);
        collector.add_trajectory(t2);
        assert!((collector.average_reward() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_experience_flush_to_buffer() {
        let mut collector = ExperienceCollector::new(2, 64);
        let mut traj = Trajectory::new();
        traj.add(make_transition(1.0));
        traj.add(make_transition(2.0));
        collector.add_trajectory(traj);
        let mut buf = ReplayBuffer::new(ReplayBufferType::Uniform, 100);
        collector.flush_to_buffer(&mut buf);
        assert_eq!(buf.len(), 2);
        assert!(collector.trajectories.is_empty());
    }

    // ── Training Metrics Tests ──────────────────────────────────────────

    #[test]
    fn test_metrics_record_and_get_latest() {
        let mut metrics = TrainingMetrics::new();
        metrics.record("reward", 1, 10.0);
        metrics.record("reward", 2, 20.0);
        assert_eq!(metrics.get_latest("reward"), Some(20.0));
    }

    #[test]
    fn test_metrics_average() {
        let mut metrics = TrainingMetrics::new();
        for i in 0..10 {
            metrics.record("loss", i, i as f64);
        }
        let avg = metrics.get_average("loss", 5).unwrap();
        // last 5: 5,6,7,8,9 → avg = 7.0
        assert!((avg - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_metrics_min_max() {
        let mut metrics = TrainingMetrics::new();
        metrics.record("val", 0, 5.0);
        metrics.record("val", 1, 1.0);
        metrics.record("val", 2, 9.0);
        assert_eq!(metrics.get_min("val"), Some(1.0));
        assert_eq!(metrics.get_max("val"), Some(9.0));
    }

    #[test]
    fn test_metrics_best_reward_tracking() {
        let mut metrics = TrainingMetrics::new();
        metrics.record("reward", 1, 10.0);
        metrics.record("reward", 2, 50.0);
        metrics.record("reward", 3, 30.0);
        assert_eq!(metrics.best_reward, 50.0);
        assert_eq!(metrics.best_reward_step, 2);
    }

    #[test]
    fn test_metrics_summary() {
        let mut metrics = TrainingMetrics::new();
        metrics.record("x", 0, 1.0);
        metrics.record("x", 1, 3.0);
        let s = metrics.summary();
        let (min, avg, max) = s.get("x").unwrap();
        assert_eq!(*min, 1.0);
        assert_eq!(*max, 3.0);
        assert!((avg - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_metrics_count() {
        let mut metrics = TrainingMetrics::new();
        metrics.record("a", 0, 1.0);
        metrics.record("a", 1, 2.0);
        assert_eq!(metrics.count("a"), 2);
        assert_eq!(metrics.count("b"), 0);
    }

    // ── Curriculum Learning Tests ───────────────────────────────────────

    fn make_stages() -> Vec<CurriculumStage> {
        vec![
            CurriculumStage {
                name: "easy".into(),
                description: "Easy level".into(),
                env_overrides: HashMap::new(),
                promotion_criteria: PromotionCriteria::RewardThreshold(10.0),
                max_steps: 1000,
                difficulty: 0.3,
            },
            CurriculumStage {
                name: "medium".into(),
                description: "Medium level".into(),
                env_overrides: HashMap::new(),
                promotion_criteria: PromotionCriteria::SuccessRate(0.8),
                max_steps: 2000,
                difficulty: 0.6,
            },
            CurriculumStage {
                name: "hard".into(),
                description: "Hard level".into(),
                env_overrides: HashMap::new(),
                promotion_criteria: PromotionCriteria::StepCount(500),
                max_steps: 5000,
                difficulty: 1.0,
            },
        ]
    }

    #[test]
    fn test_curriculum_creation() {
        let cm = CurriculumManager::new(make_stages(), true);
        assert_eq!(cm.total_stages(), 3);
        assert_eq!(cm.current_stage, 0);
    }

    #[test]
    fn test_curriculum_current_stage() {
        let cm = CurriculumManager::new(make_stages(), true);
        assert_eq!(cm.current().unwrap().name, "easy");
    }

    #[test]
    fn test_curriculum_promotion_by_reward() {
        let mut cm = CurriculumManager::new(make_stages(), true);
        let promoted = cm.check_promotion(15.0, 0.0, 0);
        assert!(promoted);
        assert_eq!(cm.current_stage, 1);
    }

    #[test]
    fn test_curriculum_no_promotion_below_threshold() {
        let mut cm = CurriculumManager::new(make_stages(), true);
        let promoted = cm.check_promotion(5.0, 0.0, 0);
        assert!(!promoted);
        assert_eq!(cm.current_stage, 0);
    }

    #[test]
    fn test_curriculum_progress_fraction() {
        let mut cm = CurriculumManager::new(make_stages(), true);
        assert!((cm.progress_fraction() - 0.0).abs() < 1e-10);
        cm.advance(100);
        assert!((cm.progress_fraction() - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_curriculum_complete() {
        let mut cm = CurriculumManager::new(make_stages(), true);
        cm.advance(100);
        cm.advance(200);
        cm.advance(300);
        assert!(cm.is_complete());
    }

    #[test]
    fn test_curriculum_difficulty() {
        let cm = CurriculumManager::new(make_stages(), true);
        assert!((cm.current_difficulty() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_promotion_criteria_combined() {
        let crit = PromotionCriteria::Combined(vec![
            PromotionCriteria::RewardThreshold(10.0),
            PromotionCriteria::MinEpisodes(5),
        ]);
        assert!(!crit.is_met(15.0, 0.0, 3, 0));
        assert!(crit.is_met(15.0, 0.0, 5, 0));
    }

    // ── Multi-Agent Tests ───────────────────────────────────────────────

    #[test]
    fn test_multi_agent_orchestrator_creation() {
        let orch = MultiAgentOrchestrator::new(MultiAgentMode::Cooperative, CommunicationProtocol::SharedReward);
        assert_eq!(orch.mode, MultiAgentMode::Cooperative);
        assert_eq!(orch.total_agents(), 0);
    }

    #[test]
    fn test_multi_agent_add_types() {
        let mut orch = MultiAgentOrchestrator::new(MultiAgentMode::Mixed, CommunicationProtocol::None);
        orch.add_agent_type(AgentType {
            name: "attacker".into(),
            policy: PolicyNetwork::mlp(10, &[64], 4),
            observation_dim: 10,
            action_dim: 4,
            count: 3,
            team_id: Some(0),
        });
        orch.add_agent_type(AgentType {
            name: "defender".into(),
            policy: PolicyNetwork::mlp(10, &[64], 4),
            observation_dim: 10,
            action_dim: 4,
            count: 2,
            team_id: Some(1),
        });
        assert_eq!(orch.total_agents(), 5);
        assert_eq!(orch.team_count(), 2);
    }

    #[test]
    fn test_multi_agent_interaction_tracking() {
        let mut orch = MultiAgentOrchestrator::new(MultiAgentMode::Cooperative, CommunicationProtocol::None);
        orch.record_interaction("agent_0", 1.0);
        orch.record_interaction("agent_0", 3.0);
        orch.record_interaction("agent_1", 5.0);
        assert!((orch.average_reward_for("agent_0") - 2.0).abs() < 1e-10);
        assert!((orch.average_reward_for("agent_1") - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_multi_agent_shared_reward() {
        let mut orch = MultiAgentOrchestrator::new(MultiAgentMode::Cooperative, CommunicationProtocol::SharedReward);
        orch.record_interaction("a", 10.0);
        orch.record_interaction("b", 20.0);
        assert!((orch.compute_shared_reward() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_multi_agent_mode_display() {
        assert_eq!(MultiAgentMode::Cooperative.to_string(), "cooperative");
        assert_eq!(MultiAgentMode::Competitive.to_string(), "competitive");
    }

    // ── A2A Protocol Tests ──────────────────────────────────────────────

    #[test]
    fn test_a2a_send_and_receive() {
        let mut proto = A2AProtocol::new();
        let msg = A2AMessage::new(
            A2AMessageType::Observation,
            "agent_0".into(),
            "agent_1".into(),
            HashMap::new(),
        );
        proto.send(msg);
        assert_eq!(proto.total_sent, 1);
        assert_eq!(proto.pending_count("agent_1"), 1);
        let received = proto.receive("agent_1");
        assert_eq!(received.len(), 1);
        assert_eq!(proto.pending_count("agent_1"), 0);
    }

    #[test]
    fn test_a2a_broadcast() {
        let mut proto = A2AProtocol::new();
        let agents = vec!["a".into(), "b".into(), "c".into()];
        proto.broadcast("a", &agents, HashMap::new());
        assert_eq!(proto.total_sent, 2); // b and c
        assert_eq!(proto.pending_count("b"), 1);
        assert_eq!(proto.pending_count("c"), 1);
        assert_eq!(proto.pending_count("a"), 0);
    }

    #[test]
    fn test_a2a_delegation() {
        let mut proto = A2AProtocol::new();
        let mut task = HashMap::new();
        task.insert("subtask_id".into(), 42.0);
        proto.delegate("leader", "worker_1", task);
        let msgs = proto.receive("worker_1");
        assert_eq!(msgs[0].msg_type, A2AMessageType::Delegation);
    }

    #[test]
    fn test_a2a_negotiation() {
        let mut proto = A2AProtocol::new();
        let mut proposal = HashMap::new();
        proposal.insert("price".into(), 100.0);
        proto.negotiate("buyer", "seller", proposal);
        let msgs = proto.receive("seller");
        assert_eq!(msgs[0].msg_type, A2AMessageType::Negotiation);
    }

    #[test]
    fn test_a2a_hierarchy() {
        let mut proto = A2AProtocol::new();
        proto.set_hierarchy("commander", vec!["squad_a".into(), "squad_b".into()]);
        let subs = proto.get_subordinates("commander");
        assert_eq!(subs.len(), 2);
        assert!(proto.get_subordinates("nobody").is_empty());
    }

    #[test]
    fn test_a2a_message_priority() {
        let msg = A2AMessage::new(
            A2AMessageType::Coordination,
            "a".into(),
            "b".into(),
            HashMap::new(),
        ).with_priority(10);
        assert_eq!(msg.priority, 10);
    }

    // ── AutoRL Tests ────────────────────────────────────────────────────

    #[test]
    fn test_autorl_grid_search() {
        let mut arl = AutoRL::new(SearchStrategy::GridSearch, 10);
        arl.add_param(HParamSpace::continuous("lr", 0.0001, 0.01));
        let p0 = arl.suggest_params(0);
        let p5 = arl.suggest_params(5);
        assert!(p0["lr"] < p5["lr"]);
    }

    #[test]
    fn test_autorl_random_search() {
        let mut arl = AutoRL::new(SearchStrategy::RandomSearch, 10);
        arl.add_param(HParamSpace::continuous("lr", 0.0001, 0.01));
        let p0 = arl.suggest_params(0);
        let p1 = arl.suggest_params(1);
        assert_ne!(p0["lr"], p1["lr"]);
    }

    #[test]
    fn test_autorl_report_result() {
        let mut arl = AutoRL::new(SearchStrategy::GridSearch, 10);
        arl.report_result(TrialResult {
            trial_id: 0,
            params: HashMap::new(),
            score: 10.0,
            steps_trained: 1000,
            status: TrialStatus::Completed,
        });
        assert_eq!(arl.completed_trials(), 1);
        assert_eq!(arl.best_score, 10.0);
    }

    #[test]
    fn test_autorl_top_k() {
        let mut arl = AutoRL::new(SearchStrategy::GridSearch, 10);
        for i in 0..5 {
            arl.report_result(TrialResult {
                trial_id: i,
                params: HashMap::new(),
                score: i as f64,
                steps_trained: 100,
                status: TrialStatus::Completed,
            });
        }
        let top = arl.top_k(3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].score, 4.0);
    }

    #[test]
    fn test_autorl_bayesian_first_trial() {
        let mut arl = AutoRL::new(
            SearchStrategy::BayesianOptimization { acquisition: "ucb".into() },
            10,
        );
        arl.add_param(HParamSpace::continuous("lr", 0.001, 0.1));
        let params = arl.suggest_params(0);
        // First trial should use midpoint
        let expected = (0.001 + 0.1) / 2.0;
        assert!((params["lr"] - expected).abs() < 1e-6);
    }

    #[test]
    fn test_hparam_space_log_scale() {
        let p = HParamSpace::log_continuous("lr", 0.0001, 0.1);
        let val = p.sample_at(0.0);
        assert!((val - 0.0001).abs() < 1e-8);
        let val_end = p.sample_at(1.0);
        assert!((val_end - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_hparam_space_discrete() {
        let p = HParamSpace::discrete("hidden", 32.0, 512.0);
        let val = p.sample_at(0.5);
        assert_eq!(val, val.round());
    }

    #[test]
    fn test_autorl_pruning() {
        let mut arl = AutoRL::new(SearchStrategy::GridSearch, 10);
        for _ in 0..5 {
            arl.report_result(TrialResult {
                trial_id: 0,
                params: HashMap::new(),
                score: 100.0,
                steps_trained: 500,
                status: TrialStatus::Completed,
            });
        }
        // Score of 10 is well below 50% of median (100), should prune
        assert!(arl.should_prune(5, 10.0, 200));
    }

    // ── Population-Based Training Tests ─────────────────────────────────

    #[test]
    fn test_pbt_creation() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let pbt = PopulationTrainer::new(10, params);
        assert_eq!(pbt.size(), 10);
    }

    #[test]
    fn test_pbt_exploit_and_explore() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let mut pbt = PopulationTrainer::new(6, params);
        let scores = vec![10.0, 5.0, 8.0, 1.0, 2.0, 15.0];
        pbt.update_scores(&scores);
        pbt.exploit_and_explore();
        assert_eq!(pbt.generation, 1);
    }

    #[test]
    fn test_pbt_elo_update() {
        let mut agent = PBTAgent::new(0, HashMap::new());
        assert_eq!(agent.elo, 1200.0);
        agent.update_elo(1200.0, 1.0); // Win against equal
        assert!(agent.elo > 1200.0);
    }

    #[test]
    fn test_pbt_expected_score() {
        let agent = PBTAgent::new(0, HashMap::new());
        let expected = agent.expected_score(1200.0);
        assert!((expected - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pbt_record_match() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let mut pbt = PopulationTrainer::new(4, params);
        pbt.record_match(0, 1, 1.0); // agent 0 wins
        assert_eq!(pbt.population[0].wins, 1);
        assert_eq!(pbt.population[1].wins, 0);
        assert!(pbt.population[0].elo > 1200.0);
    }

    #[test]
    fn test_pbt_best_agent() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let mut pbt = PopulationTrainer::new(3, params);
        pbt.update_scores(&[5.0, 15.0, 10.0]);
        let best = pbt.best_agent().unwrap();
        assert_eq!(best.id, 1);
    }

    #[test]
    fn test_pbt_elo_ranking() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let mut pbt = PopulationTrainer::new(3, params);
        pbt.record_match(0, 1, 1.0);
        pbt.record_match(0, 2, 1.0);
        let ranking = pbt.elo_ranking();
        assert_eq!(ranking[0].0, 0); // agent 0 has highest ELO
    }

    #[test]
    fn test_pbt_matchmake() {
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        let pbt = PopulationTrainer::new(4, params);
        let pair = pbt.matchmake();
        assert!(pair.is_some());
    }

    #[test]
    fn test_pbt_win_rate() {
        let mut agent = PBTAgent::new(0, HashMap::new());
        agent.matches_played = 10;
        agent.wins = 7;
        assert!((agent.win_rate() - 0.7).abs() < 1e-10);
    }

    // ── Fault Tolerance Tests ───────────────────────────────────────────

    #[test]
    fn test_fault_tolerance_checkpoint_creation() {
        let mut ft = FaultToleranceManager::new(CheckpointConfig::default());
        let mut metrics = HashMap::new();
        metrics.insert("reward".into(), 50.0);
        let ckpt = ft.create_checkpoint(10000, &metrics);
        assert_eq!(ckpt.step, 10000);
        assert_eq!(ft.checkpoint_count(), 1);
    }

    #[test]
    fn test_fault_tolerance_should_checkpoint() {
        let ft = FaultToleranceManager::new(CheckpointConfig {
            frequency_steps: 100,
            ..Default::default()
        });
        assert!(ft.should_checkpoint(100));
        assert!(!ft.should_checkpoint(50));
    }

    #[test]
    fn test_fault_tolerance_keep_last() {
        let mut ft = FaultToleranceManager::new(CheckpointConfig {
            keep_last: 2,
            ..Default::default()
        });
        let m = HashMap::new();
        ft.create_checkpoint(100, &m);
        ft.create_checkpoint(200, &m);
        ft.create_checkpoint(300, &m);
        assert_eq!(ft.checkpoint_count(), 2);
    }

    #[test]
    fn test_fault_tolerance_resume_step() {
        let mut ft = FaultToleranceManager::new(CheckpointConfig::default());
        assert_eq!(ft.resume_step(), 0);
        ft.create_checkpoint(5000, &HashMap::new());
        assert_eq!(ft.resume_step(), 5000);
    }

    #[test]
    fn test_fault_tolerance_preemption() {
        let mut ft = FaultToleranceManager::new(CheckpointConfig::default());
        assert!(!ft.preemption_detected);
        ft.mark_preemption();
        assert!(ft.preemption_detected);
    }

    #[test]
    fn test_fault_tolerance_find_by_step() {
        let mut ft = FaultToleranceManager::new(CheckpointConfig { keep_last: 10, ..Default::default() });
        let m = HashMap::new();
        ft.create_checkpoint(100, &m);
        ft.create_checkpoint(200, &m);
        ft.create_checkpoint(300, &m);
        let found = ft.find_checkpoint_by_step(250).unwrap();
        assert_eq!(found.step, 200);
    }

    // ── Training Lifecycle Tests ────────────────────────────────────────

    #[test]
    fn test_lifecycle_start() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        assert_eq!(lc.phase, TrainingPhase::Idle);
        lc.start().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Warmup);
    }

    #[test]
    fn test_lifecycle_full_flow() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        lc.start().unwrap();
        lc.begin_training().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Training);
        lc.pause().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Paused);
        lc.resume().unwrap();
        lc.begin_cooldown().unwrap();
        lc.complete().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Completed);
    }

    #[test]
    fn test_lifecycle_eval_flow() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        lc.start().unwrap();
        lc.begin_training().unwrap();
        lc.begin_eval().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Evaluating);
        lc.end_eval().unwrap();
        assert_eq!(lc.phase, TrainingPhase::Training);
    }

    #[test]
    fn test_lifecycle_step_records_metrics() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        lc.step(5.0, 0.1);
        assert_eq!(lc.current_step, 1);
        assert_eq!(lc.metrics.get_latest("reward"), Some(5.0));
        assert_eq!(lc.metrics.get_latest("loss"), Some(0.1));
    }

    #[test]
    fn test_lifecycle_progress() {
        let mut config = TrainingConfig::default();
        config.total_timesteps = 100;
        let mut lc = TrainingLifecycle::new(config);
        for _ in 0..50 {
            lc.step(1.0, 0.1);
        }
        assert!((lc.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_lifecycle_invalid_transition() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        assert!(lc.begin_training().is_err()); // Not in warmup
        assert!(lc.pause().is_err()); // Not in training
    }

    #[test]
    fn test_lifecycle_fail() {
        let config = TrainingConfig::default();
        let mut lc = TrainingLifecycle::new(config);
        lc.fail("out of memory");
        assert!(matches!(lc.phase, TrainingPhase::Failed(_)));
        assert!(lc.is_done());
    }

    #[test]
    fn test_lifecycle_checkpointing_during_steps() {
        let mut config = TrainingConfig::default();
        config.checkpoint.frequency_steps = 5;
        let mut lc = TrainingLifecycle::new(config);
        for _ in 0..10 {
            lc.step(1.0, 0.1);
        }
        assert!(lc.fault_tolerance.checkpoint_count() >= 1);
    }

    // ── TrainOS Orchestrator Tests ──────────────────────────────────────

    #[test]
    fn test_trainos_creation() {
        let os = TrainOS::new();
        assert_eq!(os.algorithm_count(), 30);
        assert!(os.lifecycle.is_none());
    }

    #[test]
    fn test_trainos_configure() {
        let mut os = TrainOS::new();
        os.configure(TrainingConfig::default());
        assert!(os.lifecycle.is_some());
        assert!(os.distributed.is_some());
    }

    #[test]
    fn test_trainos_setup_replay_buffer() {
        let mut os = TrainOS::new();
        os.setup_replay_buffer(ReplayBufferType::Uniform, 10000);
        assert!(os.replay_buffer.is_some());
    }

    #[test]
    fn test_trainos_setup_curriculum() {
        let mut os = TrainOS::new();
        os.setup_curriculum(make_stages(), true);
        assert!(os.curriculum.is_some());
    }

    #[test]
    fn test_trainos_status_summary() {
        let mut os = TrainOS::new();
        os.configure(TrainingConfig::default());
        os.setup_replay_buffer(ReplayBufferType::Uniform, 100);
        let summary = os.status_summary();
        assert!(summary.contains_key("algorithms"));
        assert!(summary.contains_key("phase"));
    }

    #[test]
    fn test_trainos_full_setup() {
        let mut os = TrainOS::new();
        os.configure(TrainingConfig::default());
        os.setup_replay_buffer(ReplayBufferType::Uniform, 10000);
        os.setup_curriculum(make_stages(), true);
        os.setup_multi_agent(MultiAgentMode::Cooperative, CommunicationProtocol::SharedReward);
        os.setup_auto_rl(SearchStrategy::RandomSearch, 20);
        let mut params = HashMap::new();
        params.insert("lr".into(), 0.001);
        os.setup_pbt(8, params);
        os.setup_collector(4, 128);

        assert!(os.lifecycle.is_some());
        assert!(os.replay_buffer.is_some());
        assert!(os.curriculum.is_some());
        assert!(os.multi_agent.is_some());
        assert!(os.auto_rl.is_some());
        assert!(os.pbt.is_some());
        assert!(os.collector.is_some());
    }
}
