#![allow(dead_code)]
//! RL-OS Environment Operating System — full reinforcement learning environment management.
//!
//! Provides a production-grade RL environment stack:
//! - Environment definition DSL with YAML-like parsing (observation/action spaces, reward, connectors)
//! - Space types: Box, Discrete, MultiDiscrete, MultiBinary, Dict, Tuple with validation
//! - Git-like environment versioning (commit, diff, rollback, tag)
//! - Simulation backend abstraction (MuJoCo, PhysX, Brax, Unity, Custom)
//! - Real-world connectors (REST, gRPC, MQTT, WebSocket, DB) with retry/circuit-breaker
//! - Time-travel replay with deterministic checkpoints and trajectory recording
//! - Domain randomization (uniform, normal, lognormal) with per-parameter configs
//! - Hybrid sim+real training pipeline with domain adaptation tracking
//! - Environment registry for discovery and metadata search
//! - Pluggable reward function library (Sharpe, L2, energy penalty, custom)
//! - Vectorized environments (sync/async parallel stepping)
//! - Safety constraints (position limits, collision avoidance, regulatory) with violation tracking
//! - Gymnasium/PettingZoo compatibility wrappers
//! - Finance-specific: market data, order book simulation, regulatory constraints
//! - Robotics-specific: sim-to-real, ROS 2 bridge, joint limits

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// §1  Space Types
// ══════════════════════════════════════════════════════════════════════════════

/// Distribution type for domain randomization.
#[derive(Debug, Clone, PartialEq)]
pub enum Distribution {
    Uniform { low: f64, high: f64 },
    Normal { mean: f64, std_dev: f64 },
    LogNormal { mu: f64, sigma: f64 },
    Categorical { probs: Vec<f64> },
}

impl Distribution {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Uniform { .. } => "Uniform",
            Self::Normal { .. } => "Normal",
            Self::LogNormal { .. } => "LogNormal",
            Self::Categorical { .. } => "Categorical",
        }
    }

    /// Validate distribution parameters.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Uniform { low, high } => {
                if low >= high {
                    return Err(format!("Uniform: low ({low}) must be < high ({high})"));
                }
                Ok(())
            }
            Self::Normal { std_dev, .. } => {
                if *std_dev <= 0.0 {
                    return Err(format!("Normal: std_dev ({std_dev}) must be > 0"));
                }
                Ok(())
            }
            Self::LogNormal { sigma, .. } => {
                if *sigma <= 0.0 {
                    return Err(format!("LogNormal: sigma ({sigma}) must be > 0"));
                }
                Ok(())
            }
            Self::Categorical { probs } => {
                if probs.is_empty() {
                    return Err("Categorical: probs must be non-empty".into());
                }
                let sum: f64 = probs.iter().sum();
                if (sum - 1.0).abs() > 1e-6 {
                    return Err(format!("Categorical: probs sum to {sum}, expected 1.0"));
                }
                Ok(())
            }
        }
    }

    /// Sample a value (deterministic pseudo-sample using a seed for reproducibility).
    pub fn sample_deterministic(&self, seed: u64) -> f64 {
        // Simple LCG-based pseudo-random for deterministic replay
        let t = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) as f64)
            / (u64::MAX as f64);
        match self {
            Self::Uniform { low, high } => low + t * (high - low),
            Self::Normal { mean, std_dev } => {
                // Box-Muller approximation using single uniform
                let u = t.max(1e-10);
                let z = (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * t).cos();
                mean + std_dev * z
            }
            Self::LogNormal { mu, sigma } => {
                let u = t.max(1e-10);
                let z = (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * t).cos();
                (mu + sigma * z).exp()
            }
            Self::Categorical { probs } => {
                let mut cumulative = 0.0;
                for (i, p) in probs.iter().enumerate() {
                    cumulative += p;
                    if t < cumulative {
                        return i as f64;
                    }
                }
                (probs.len() - 1) as f64
            }
        }
    }
}

impl std::fmt::Display for Distribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uniform { low, high } => write!(f, "Uniform({low}, {high})"),
            Self::Normal { mean, std_dev } => write!(f, "Normal({mean}, {std_dev})"),
            Self::LogNormal { mu, sigma } => write!(f, "LogNormal({mu}, {sigma})"),
            Self::Categorical { probs } => write!(f, "Categorical({probs:?})"),
        }
    }
}

/// Data type for space elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DType {
    Float32,
    Float64,
    Int32,
    Int64,
    Bool,
    UInt8,
}

impl DType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Float32 => "float32",
            Self::Float64 => "float64",
            Self::Int32 => "int32",
            Self::Int64 => "int64",
            Self::Bool => "bool",
            Self::UInt8 => "uint8",
        }
    }

    pub fn byte_size(&self) -> usize {
        match self {
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::Int32 => 4,
            Self::Int64 => 8,
            Self::Bool => 1,
            Self::UInt8 => 1,
        }
    }
}

impl std::fmt::Display for DType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// RL observation/action space definition (Gymnasium-compatible).
#[derive(Debug, Clone, PartialEq)]
pub enum SpaceType {
    /// Continuous n-dimensional box with lower/upper bounds.
    Box {
        low: Vec<f64>,
        high: Vec<f64>,
        shape: Vec<usize>,
        dtype: DType,
    },
    /// Single integer in [0, n).
    Discrete {
        n: u64,
    },
    /// Vector of discrete values, each in [0, nvec[i]).
    MultiDiscrete {
        nvec: Vec<u64>,
    },
    /// Binary vector of length n.
    MultiBinary {
        n: usize,
    },
    /// Dictionary of named sub-spaces.
    Dict {
        spaces: HashMap<String, SpaceType>,
    },
    /// Ordered tuple of sub-spaces.
    Tuple {
        spaces: Vec<SpaceType>,
    },
}

impl SpaceType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Box { .. } => "Box",
            Self::Discrete { .. } => "Discrete",
            Self::MultiDiscrete { .. } => "MultiDiscrete",
            Self::MultiBinary { .. } => "MultiBinary",
            Self::Dict { .. } => "Dict",
            Self::Tuple { .. } => "Tuple",
        }
    }

    /// Total number of scalar elements in this space.
    pub fn flat_size(&self) -> usize {
        match self {
            Self::Box { shape, .. } => shape.iter().product::<usize>().max(1),
            Self::Discrete { .. } => 1,
            Self::MultiDiscrete { nvec } => nvec.len(),
            Self::MultiBinary { n } => *n,
            Self::Dict { spaces } => spaces.values().map(|s| s.flat_size()).sum(),
            Self::Tuple { spaces } => spaces.iter().map(|s| s.flat_size()).sum(),
        }
    }

    /// Validate the space definition.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Box { low, high, shape, .. } => {
                let expected: usize = shape.iter().product::<usize>().max(1);
                if low.len() != expected {
                    return Err(format!(
                        "Box: low length {} != shape product {expected}",
                        low.len()
                    ));
                }
                if high.len() != expected {
                    return Err(format!(
                        "Box: high length {} != shape product {expected}",
                        high.len()
                    ));
                }
                for (i, (l, h)) in low.iter().zip(high.iter()).enumerate() {
                    if l > h {
                        return Err(format!("Box: low[{i}]={l} > high[{i}]={h}"));
                    }
                }
                Ok(())
            }
            Self::Discrete { n } => {
                if *n == 0 {
                    return Err("Discrete: n must be > 0".into());
                }
                Ok(())
            }
            Self::MultiDiscrete { nvec } => {
                if nvec.is_empty() {
                    return Err("MultiDiscrete: nvec must be non-empty".into());
                }
                for (i, v) in nvec.iter().enumerate() {
                    if *v == 0 {
                        return Err(format!("MultiDiscrete: nvec[{i}] must be > 0"));
                    }
                }
                Ok(())
            }
            Self::MultiBinary { n } => {
                if *n == 0 {
                    return Err("MultiBinary: n must be > 0".into());
                }
                Ok(())
            }
            Self::Dict { spaces } => {
                if spaces.is_empty() {
                    return Err("Dict: must contain at least one space".into());
                }
                for (name, space) in spaces {
                    space
                        .validate()
                        .map_err(|e| format!("Dict[{name}]: {e}"))?;
                }
                Ok(())
            }
            Self::Tuple { spaces } => {
                if spaces.is_empty() {
                    return Err("Tuple: must contain at least one space".into());
                }
                for (i, space) in spaces.iter().enumerate() {
                    space
                        .validate()
                        .map_err(|e| format!("Tuple[{i}]: {e}"))?;
                }
                Ok(())
            }
        }
    }

    /// Check whether a flat vector of f64 values is within this space.
    pub fn contains(&self, values: &[f64]) -> bool {
        match self {
            Self::Box { low, high, .. } => {
                if values.len() != low.len() {
                    return false;
                }
                values
                    .iter()
                    .zip(low.iter().zip(high.iter()))
                    .all(|(v, (l, h))| *v >= *l && *v <= *h)
            }
            Self::Discrete { n } => {
                values.len() == 1 && values[0] >= 0.0 && values[0] < (*n as f64)
                    && (values[0] - values[0].round()).abs() < 1e-9
            }
            Self::MultiDiscrete { nvec } => {
                if values.len() != nvec.len() {
                    return false;
                }
                values
                    .iter()
                    .zip(nvec.iter())
                    .all(|(v, n)| *v >= 0.0 && *v < (*n as f64) && (v - v.round()).abs() < 1e-9)
            }
            Self::MultiBinary { n } => {
                if values.len() != *n {
                    return false;
                }
                values.iter().all(|v| *v == 0.0 || *v == 1.0)
            }
            _ => true, // Dict/Tuple need structured checking
        }
    }
}

impl std::fmt::Display for SpaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Box { shape, dtype, .. } => write!(f, "Box({shape:?}, {dtype})"),
            Self::Discrete { n } => write!(f, "Discrete({n})"),
            Self::MultiDiscrete { nvec } => write!(f, "MultiDiscrete({nvec:?})"),
            Self::MultiBinary { n } => write!(f, "MultiBinary({n})"),
            Self::Dict { spaces } => {
                let keys: Vec<&String> = spaces.keys().collect();
                write!(f, "Dict({keys:?})")
            }
            Self::Tuple { spaces } => write!(f, "Tuple(len={})", spaces.len()),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §2  Reward Functions
// ══════════════════════════════════════════════════════════════════════════════

/// Built-in reward function types.
#[derive(Debug, Clone, PartialEq)]
pub enum RewardFunctionType {
    /// Sharpe ratio: mean(returns)/std(returns)
    SharpeRatio,
    /// L2 distance to target: -||state - target||_2
    L2Distance { target: Vec<f64> },
    /// Energy penalty: -weight * sum(action^2)
    EnergyPenalty { weight: f64 },
    /// Sparse: 1.0 on goal, 0.0 otherwise
    Sparse { goal_threshold: f64 },
    /// Dense: negative distance to goal
    Dense { goal: Vec<f64> },
    /// Weighted sum of sub-rewards
    Composite { components: Vec<(String, f64)> },
    /// Custom expression (parsed at runtime)
    Custom { expression: String },
}

impl RewardFunctionType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SharpeRatio => "SharpeRatio",
            Self::L2Distance { .. } => "L2Distance",
            Self::EnergyPenalty { .. } => "EnergyPenalty",
            Self::Sparse { .. } => "Sparse",
            Self::Dense { .. } => "Dense",
            Self::Composite { .. } => "Composite",
            Self::Custom { .. } => "Custom",
        }
    }
}

impl std::fmt::Display for RewardFunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A reward component with weight and function.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardComponent {
    pub name: String,
    pub weight: f64,
    pub function: RewardFunctionType,
    pub clip_min: Option<f64>,
    pub clip_max: Option<f64>,
}

impl RewardComponent {
    pub fn new(name: &str, weight: f64, function: RewardFunctionType) -> Self {
        Self {
            name: name.to_string(),
            weight,
            function,
            clip_min: None,
            clip_max: None,
        }
    }

    pub fn with_clip(mut self, min: f64, max: f64) -> Self {
        self.clip_min = Some(min);
        self.clip_max = Some(max);
        self
    }

    /// Compute reward for given state/action/returns.
    pub fn compute(&self, state: &[f64], action: &[f64], returns: &[f64]) -> f64 {
        let raw = match &self.function {
            RewardFunctionType::SharpeRatio => {
                if returns.is_empty() {
                    return 0.0;
                }
                let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                    / returns.len() as f64;
                let std = var.sqrt();
                if std < 1e-10 {
                    0.0
                } else {
                    mean / std
                }
            }
            RewardFunctionType::L2Distance { target } => {
                let dist: f64 = state
                    .iter()
                    .zip(target.iter())
                    .map(|(s, t)| (s - t).powi(2))
                    .sum::<f64>()
                    .sqrt();
                -dist
            }
            RewardFunctionType::EnergyPenalty { weight } => {
                let energy: f64 = action.iter().map(|a| a.powi(2)).sum();
                -weight * energy
            }
            RewardFunctionType::Sparse { goal_threshold } => {
                let dist: f64 = state.iter().map(|s| s.powi(2)).sum::<f64>().sqrt();
                if dist < *goal_threshold {
                    1.0
                } else {
                    0.0
                }
            }
            RewardFunctionType::Dense { goal } => {
                let dist: f64 = state
                    .iter()
                    .zip(goal.iter())
                    .map(|(s, g)| (s - g).powi(2))
                    .sum::<f64>()
                    .sqrt();
                -dist
            }
            RewardFunctionType::Composite { .. } => 0.0, // needs sub-evaluation
            RewardFunctionType::Custom { .. } => 0.0,     // needs runtime parser
        };

        let clipped = match (self.clip_min, self.clip_max) {
            (Some(lo), Some(hi)) => raw.clamp(lo, hi),
            (Some(lo), None) => raw.max(lo),
            (None, Some(hi)) => raw.min(hi),
            (None, None) => raw,
        };

        self.weight * clipped
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §3  Safety Constraints
// ══════════════════════════════════════════════════════════════════════════════

/// Constraint severity level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintSeverity {
    Warning,
    Hard,
    Critical,
}

impl ConstraintSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Hard => "Hard",
            Self::Critical => "Critical",
        }
    }

    pub fn penalty_multiplier(&self) -> f64 {
        match self {
            Self::Warning => 0.1,
            Self::Hard => 1.0,
            Self::Critical => 10.0,
        }
    }
}

impl std::fmt::Display for ConstraintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Safety constraint types.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyConstraintType {
    /// State dimension must stay within bounds.
    PositionLimit {
        dimension: usize,
        min: f64,
        max: f64,
    },
    /// Velocity must not exceed maximum.
    VelocityLimit {
        dimension: usize,
        max_speed: f64,
    },
    /// Minimum distance between two entities.
    CollisionAvoidance {
        entity_a: String,
        entity_b: String,
        min_distance: f64,
    },
    /// Torque/force limit on an actuator.
    TorqueLimit {
        joint_index: usize,
        max_torque: f64,
    },
    /// Regulatory compliance rule.
    Regulatory {
        rule_id: String,
        description: String,
    },
    /// Custom expression constraint.
    CustomExpression {
        expression: String,
    },
}

impl SafetyConstraintType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PositionLimit { .. } => "PositionLimit",
            Self::VelocityLimit { .. } => "VelocityLimit",
            Self::CollisionAvoidance { .. } => "CollisionAvoidance",
            Self::TorqueLimit { .. } => "TorqueLimit",
            Self::Regulatory { .. } => "Regulatory",
            Self::CustomExpression { .. } => "CustomExpression",
        }
    }
}

impl std::fmt::Display for SafetyConstraintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A safety constraint with violation tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyConstraint {
    pub name: String,
    pub constraint_type: SafetyConstraintType,
    pub severity: ConstraintSeverity,
    pub violation_count: u64,
    pub enabled: bool,
}

impl SafetyConstraint {
    pub fn new(name: &str, constraint_type: SafetyConstraintType, severity: ConstraintSeverity) -> Self {
        Self {
            name: name.to_string(),
            constraint_type,
            severity,
            violation_count: 0,
            enabled: true,
        }
    }

    /// Check if the constraint is violated by the given state/action.
    pub fn check(&mut self, state: &[f64], action: &[f64]) -> Option<ConstraintViolation> {
        if !self.enabled {
            return None;
        }
        let violated = match &self.constraint_type {
            SafetyConstraintType::PositionLimit { dimension, min, max } => {
                if let Some(val) = state.get(*dimension) {
                    *val < *min || *val > *max
                } else {
                    false
                }
            }
            SafetyConstraintType::VelocityLimit { dimension, max_speed } => {
                if let Some(val) = state.get(*dimension) {
                    val.abs() > *max_speed
                } else {
                    false
                }
            }
            SafetyConstraintType::TorqueLimit { joint_index, max_torque } => {
                if let Some(val) = action.get(*joint_index) {
                    val.abs() > *max_torque
                } else {
                    false
                }
            }
            SafetyConstraintType::CollisionAvoidance { min_distance, .. } => {
                // Simplified: assume first two dims encode distance
                if state.len() >= 2 {
                    let dist = (state[0].powi(2) + state[1].powi(2)).sqrt();
                    dist < *min_distance
                } else {
                    false
                }
            }
            SafetyConstraintType::Regulatory { .. } => false,
            SafetyConstraintType::CustomExpression { .. } => false,
        };

        if violated {
            self.violation_count += 1;
            Some(ConstraintViolation {
                constraint_name: self.name.clone(),
                severity: self.severity.clone(),
                violation_number: self.violation_count,
            })
        } else {
            None
        }
    }

    pub fn reset_violations(&mut self) {
        self.violation_count = 0;
    }
}

/// Record of a constraint violation.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintViolation {
    pub constraint_name: String,
    pub severity: ConstraintSeverity,
    pub violation_number: u64,
}

// ══════════════════════════════════════════════════════════════════════════════
// §4  Simulation Backends
// ══════════════════════════════════════════════════════════════════════════════

/// Simulation backend type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimBackendKind {
    MuJoCo,
    PhysX,
    Brax,
    Unity,
    Custom,
}

impl SimBackendKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::MuJoCo => "MuJoCo",
            Self::PhysX => "PhysX",
            Self::Brax => "Brax",
            Self::Unity => "Unity",
            Self::Custom => "Custom",
        }
    }

    pub fn supports_gpu(&self) -> bool {
        matches!(self, Self::Brax | Self::PhysX | Self::Unity)
    }

    pub fn supports_deterministic(&self) -> bool {
        matches!(self, Self::MuJoCo | Self::Brax | Self::Custom)
    }
}

impl std::fmt::Display for SimBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Configuration for a simulation backend.
#[derive(Debug, Clone, PartialEq)]
pub struct SimBackendConfig {
    pub kind: SimBackendKind,
    pub timestep: f64,
    pub substeps: u32,
    pub gravity: [f64; 3],
    pub solver_iterations: u32,
    pub gpu_enabled: bool,
    pub deterministic: bool,
    pub custom_params: HashMap<String, String>,
}

impl SimBackendConfig {
    pub fn new(kind: SimBackendKind) -> Self {
        Self {
            kind,
            timestep: 0.002,
            substeps: 4,
            gravity: [0.0, 0.0, -9.81],
            solver_iterations: 50,
            gpu_enabled: false,
            deterministic: true,
            custom_params: HashMap::new(),
        }
    }

    pub fn with_timestep(mut self, dt: f64) -> Self {
        self.timestep = dt;
        self
    }

    pub fn with_gravity(mut self, g: [f64; 3]) -> Self {
        self.gravity = g;
        self
    }

    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.gpu_enabled = enabled;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.timestep <= 0.0 {
            return Err("timestep must be > 0".into());
        }
        if self.substeps == 0 {
            return Err("substeps must be > 0".into());
        }
        if self.gpu_enabled && !self.kind.supports_gpu() {
            return Err(format!("{} does not support GPU", self.kind));
        }
        if self.deterministic && !self.kind.supports_deterministic() {
            return Err(format!("{} does not support deterministic mode", self.kind));
        }
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §5  Connector Types (Real-World)
// ══════════════════════════════════════════════════════════════════════════════

/// Connector protocol type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectorProtocol {
    RestApi,
    Grpc,
    Mqtt,
    WebSocket,
    Database,
}

impl ConnectorProtocol {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RestApi => "REST",
            Self::Grpc => "gRPC",
            Self::Mqtt => "MQTT",
            Self::WebSocket => "WebSocket",
            Self::Database => "Database",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            Self::RestApi => 8080,
            Self::Grpc => 50051,
            Self::Mqtt => 1883,
            Self::WebSocket => 8765,
            Self::Database => 5432,
        }
    }
}

impl std::fmt::Display for ConnectorProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Circuit breaker state for connector resilience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreakerState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Closed => "Closed",
            Self::Open => "Open",
            Self::HalfOpen => "HalfOpen",
        }
    }
}

impl std::fmt::Display for CircuitBreakerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Circuit breaker for resilient connectors.
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitBreaker {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub success_count: u32,
    pub success_threshold: u32,
    pub half_open_max_calls: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            success_threshold,
            half_open_max_calls: 3,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitBreakerState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.success_count = 0;
            }
            CircuitBreakerState::Open => {}
        }
    }

    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => false,
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub fn try_half_open(&mut self) {
        if self.state == CircuitBreakerState::Open {
            self.state = CircuitBreakerState::HalfOpen;
            self.success_count = 0;
        }
    }

    pub fn reset(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
    }
}

/// Retry configuration for connectors.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl RetryConfig {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            base_delay_ms: 100,
            max_delay_ms: 30_000,
            exponential_backoff: true,
        }
    }

    /// Compute delay for the nth retry.
    pub fn delay_for_retry(&self, retry_num: u32) -> u64 {
        if self.exponential_backoff {
            let delay = self.base_delay_ms * 2u64.pow(retry_num);
            delay.min(self.max_delay_ms)
        } else {
            self.base_delay_ms
        }
    }
}

/// Real-world connector configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectorConfig {
    pub name: String,
    pub protocol: ConnectorProtocol,
    pub endpoint: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub retry: RetryConfig,
    pub circuit_breaker: CircuitBreaker,
    pub headers: HashMap<String, String>,
    pub enabled: bool,
}

impl ConnectorConfig {
    pub fn new(name: &str, protocol: ConnectorProtocol, endpoint: &str) -> Self {
        let port = protocol.default_port();
        Self {
            name: name.to_string(),
            protocol,
            endpoint: endpoint.to_string(),
            port,
            timeout_ms: 5000,
            retry: RetryConfig::new(3),
            circuit_breaker: CircuitBreaker::new(5, 3),
            headers: HashMap::new(),
            enabled: true,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Connector name must not be empty".into());
        }
        if self.endpoint.is_empty() {
            return Err("Connector endpoint must not be empty".into());
        }
        if self.timeout_ms == 0 {
            return Err("Timeout must be > 0".into());
        }
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §6  Domain Randomization
// ══════════════════════════════════════════════════════════════════════════════

/// A single parameter randomization config.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomizationParam {
    pub name: String,
    pub distribution: Distribution,
    pub enabled: bool,
    pub apply_every_n_episodes: u64,
}

impl RandomizationParam {
    pub fn new(name: &str, distribution: Distribution) -> Self {
        Self {
            name: name.to_string(),
            distribution,
            enabled: true,
            apply_every_n_episodes: 1,
        }
    }

    pub fn should_apply(&self, episode: u64) -> bool {
        self.enabled && (episode % self.apply_every_n_episodes == 0)
    }

    pub fn sample(&self, seed: u64) -> f64 {
        self.distribution.sample_deterministic(seed)
    }
}

/// Domain randomization configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainRandomization {
    pub params: Vec<RandomizationParam>,
    pub seed: u64,
    pub enabled: bool,
}

impl DomainRandomization {
    pub fn new(seed: u64) -> Self {
        Self {
            params: Vec::new(),
            seed,
            enabled: true,
        }
    }

    pub fn add_param(&mut self, param: RandomizationParam) {
        self.params.push(param);
    }

    pub fn active_params(&self) -> Vec<&RandomizationParam> {
        self.params.iter().filter(|p| p.enabled).collect()
    }

    /// Generate randomized parameter values for a given episode.
    pub fn sample_all(&self, episode: u64) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        if !self.enabled {
            return result;
        }
        for (i, param) in self.params.iter().enumerate() {
            if param.should_apply(episode) {
                let seed = self.seed.wrapping_add(episode).wrapping_mul(i as u64 + 1);
                result.insert(param.name.clone(), param.sample(seed));
            }
        }
        result
    }

    pub fn validate(&self) -> Result<(), String> {
        for param in &self.params {
            param.distribution.validate().map_err(|e| {
                format!("Randomization param '{}': {e}", param.name)
            })?;
        }
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §7  Time-Travel Replay
// ══════════════════════════════════════════════════════════════════════════════

/// A snapshot of environment state at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct StateSnapshot {
    pub step: u64,
    pub state: Vec<f64>,
    pub action: Vec<f64>,
    pub reward: f64,
    pub done: bool,
    pub info: HashMap<String, String>,
}

impl StateSnapshot {
    pub fn new(step: u64, state: Vec<f64>, action: Vec<f64>, reward: f64, done: bool) -> Self {
        Self {
            step,
            state,
            action,
            reward,
            done,
            info: HashMap::new(),
        }
    }
}

/// A trajectory: ordered sequence of snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct Trajectory {
    pub id: String,
    pub snapshots: Vec<StateSnapshot>,
    pub total_reward: f64,
    pub metadata: HashMap<String, String>,
}

impl Trajectory {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            snapshots: Vec::new(),
            total_reward: 0.0,
            metadata: HashMap::new(),
        }
    }

    pub fn add_snapshot(&mut self, snapshot: StateSnapshot) {
        self.total_reward += snapshot.reward;
        self.snapshots.push(snapshot);
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Get snapshot at a specific step.
    pub fn at_step(&self, step: u64) -> Option<&StateSnapshot> {
        self.snapshots.iter().find(|s| s.step == step)
    }

    /// Replay from a given step index, returning remaining snapshots.
    pub fn replay_from(&self, step: u64) -> Vec<&StateSnapshot> {
        self.snapshots.iter().filter(|s| s.step >= step).collect()
    }

    /// Compute discounted return from a step.
    pub fn discounted_return(&self, from_step: u64, gamma: f64) -> f64 {
        let mut ret = 0.0;
        let mut discount = 1.0;
        for snap in self.snapshots.iter().filter(|s| s.step >= from_step) {
            ret += discount * snap.reward;
            discount *= gamma;
        }
        ret
    }

    /// Average reward per step.
    pub fn average_reward(&self) -> f64 {
        if self.snapshots.is_empty() {
            return 0.0;
        }
        self.total_reward / self.snapshots.len() as f64
    }
}

/// Time-travel replay manager.
#[derive(Debug, Clone)]
pub struct ReplayManager {
    pub trajectories: HashMap<String, Trajectory>,
    pub checkpoints: HashMap<String, Vec<StateSnapshot>>,
    pub max_trajectories: usize,
}

impl ReplayManager {
    pub fn new(max_trajectories: usize) -> Self {
        Self {
            trajectories: HashMap::new(),
            checkpoints: HashMap::new(),
            max_trajectories,
        }
    }

    pub fn start_trajectory(&mut self, id: &str) -> Result<(), String> {
        if self.trajectories.len() >= self.max_trajectories {
            return Err(format!("Max trajectories ({}) reached", self.max_trajectories));
        }
        self.trajectories.insert(id.to_string(), Trajectory::new(id));
        Ok(())
    }

    pub fn record_step(
        &mut self,
        trajectory_id: &str,
        snapshot: StateSnapshot,
    ) -> Result<(), String> {
        let traj = self
            .trajectories
            .get_mut(trajectory_id)
            .ok_or_else(|| format!("Trajectory '{trajectory_id}' not found"))?;
        traj.add_snapshot(snapshot);
        Ok(())
    }

    pub fn save_checkpoint(&mut self, name: &str, trajectory_id: &str) -> Result<(), String> {
        let traj = self
            .trajectories
            .get(trajectory_id)
            .ok_or_else(|| format!("Trajectory '{trajectory_id}' not found"))?;
        self.checkpoints
            .insert(name.to_string(), traj.snapshots.clone());
        Ok(())
    }

    pub fn get_trajectory(&self, id: &str) -> Option<&Trajectory> {
        self.trajectories.get(id)
    }

    pub fn list_trajectories(&self) -> Vec<&str> {
        self.trajectories.keys().map(|k| k.as_str()).collect()
    }

    pub fn total_steps(&self) -> u64 {
        self.trajectories.values().map(|t| t.len() as u64).sum()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §8  Environment Versioning
// ══════════════════════════════════════════════════════════════════════════════

/// A version commit for an environment definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvCommit {
    pub hash: String,
    pub message: String,
    pub parent_hash: Option<String>,
    pub timestamp: u64,
    pub snapshot: String, // serialized env definition
}

impl EnvCommit {
    pub fn new(hash: &str, message: &str, parent: Option<&str>, timestamp: u64, snapshot: &str) -> Self {
        Self {
            hash: hash.to_string(),
            message: message.to_string(),
            parent_hash: parent.map(|s| s.to_string()),
            timestamp,
            snapshot: snapshot.to_string(),
        }
    }
}

/// A named tag pointing to a commit.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvTag {
    pub name: String,
    pub commit_hash: String,
    pub message: String,
}

/// Environment version control system.
#[derive(Debug, Clone)]
pub struct EnvVersionControl {
    pub commits: Vec<EnvCommit>,
    pub tags: HashMap<String, EnvTag>,
    pub head: Option<String>,
    pub branch: String,
}

impl EnvVersionControl {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            tags: HashMap::new(),
            head: None,
            branch: "main".to_string(),
        }
    }

    /// Generate a simple hash from content.
    fn hash_content(content: &str, timestamp: u64) -> String {
        let mut h: u64 = 5381;
        for byte in content.bytes() {
            h = h.wrapping_mul(33).wrapping_add(byte as u64);
        }
        h = h.wrapping_add(timestamp);
        format!("{:016x}", h)
    }

    pub fn commit(&mut self, message: &str, snapshot: &str, timestamp: u64) -> String {
        let hash = Self::hash_content(snapshot, timestamp);
        let commit = EnvCommit::new(
            &hash,
            message,
            self.head.as_deref(),
            timestamp,
            snapshot,
        );
        self.commits.push(commit);
        self.head = Some(hash.clone());
        hash
    }

    pub fn tag(&mut self, tag_name: &str, message: &str) -> Result<(), String> {
        let head = self
            .head
            .as_ref()
            .ok_or("No commits to tag")?
            .clone();
        if self.tags.contains_key(tag_name) {
            return Err(format!("Tag '{tag_name}' already exists"));
        }
        self.tags.insert(
            tag_name.to_string(),
            EnvTag {
                name: tag_name.to_string(),
                commit_hash: head,
                message: message.to_string(),
            },
        );
        Ok(())
    }

    pub fn rollback(&mut self, commit_hash: &str) -> Result<&EnvCommit, String> {
        let idx = self
            .commits
            .iter()
            .position(|c| c.hash == commit_hash)
            .ok_or_else(|| format!("Commit '{commit_hash}' not found"))?;
        self.head = Some(commit_hash.to_string());
        // Truncate future commits (branch reset)
        self.commits.truncate(idx + 1);
        Ok(&self.commits[idx])
    }

    /// Diff two commits (simple line-based diff of snapshots).
    pub fn diff(&self, hash_a: &str, hash_b: &str) -> Result<Vec<String>, String> {
        let a = self
            .commits
            .iter()
            .find(|c| c.hash == hash_a)
            .ok_or_else(|| format!("Commit '{hash_a}' not found"))?;
        let b = self
            .commits
            .iter()
            .find(|c| c.hash == hash_b)
            .ok_or_else(|| format!("Commit '{hash_b}' not found"))?;

        let lines_a: Vec<&str> = a.snapshot.lines().collect();
        let lines_b: Vec<&str> = b.snapshot.lines().collect();
        let mut diffs = Vec::new();
        let max_len = lines_a.len().max(lines_b.len());
        for i in 0..max_len {
            let la = lines_a.get(i).copied().unwrap_or("");
            let lb = lines_b.get(i).copied().unwrap_or("");
            if la != lb {
                diffs.push(format!("@@ line {} @@\n- {la}\n+ {lb}", i + 1));
            }
        }
        Ok(diffs)
    }

    pub fn log(&self) -> Vec<&EnvCommit> {
        self.commits.iter().rev().collect()
    }

    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    pub fn get_commit(&self, hash: &str) -> Option<&EnvCommit> {
        self.commits.iter().find(|c| c.hash == hash)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §9  Environment Definition
// ══════════════════════════════════════════════════════════════════════════════

/// Training mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrainingMode {
    Simulation,
    RealWorld,
    HybridSimReal,
}

impl TrainingMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Simulation => "Simulation",
            Self::RealWorld => "RealWorld",
            Self::HybridSimReal => "HybridSimReal",
        }
    }
}

impl std::fmt::Display for TrainingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Full environment definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentDef {
    pub name: String,
    pub version: String,
    pub description: String,
    pub observation_space: SpaceType,
    pub action_space: SpaceType,
    pub reward_components: Vec<RewardComponent>,
    pub safety_constraints: Vec<SafetyConstraint>,
    pub sim_backend: Option<SimBackendConfig>,
    pub connectors: Vec<ConnectorConfig>,
    pub randomization: Option<DomainRandomization>,
    pub training_mode: TrainingMode,
    pub max_episode_steps: u64,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
}

impl EnvironmentDef {
    pub fn new(name: &str, obs_space: SpaceType, act_space: SpaceType) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: String::new(),
            observation_space: obs_space,
            action_space: act_space,
            reward_components: Vec::new(),
            safety_constraints: Vec::new(),
            sim_backend: None,
            connectors: Vec::new(),
            randomization: None,
            training_mode: TrainingMode::Simulation,
            max_episode_steps: 1000,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_backend(mut self, backend: SimBackendConfig) -> Self {
        self.sim_backend = Some(backend);
        self
    }

    pub fn with_training_mode(mut self, mode: TrainingMode) -> Self {
        self.training_mode = mode;
        self
    }

    pub fn add_reward(&mut self, component: RewardComponent) {
        self.reward_components.push(component);
    }

    pub fn add_constraint(&mut self, constraint: SafetyConstraint) {
        self.safety_constraints.push(constraint);
    }

    pub fn add_connector(&mut self, connector: ConnectorConfig) {
        self.connectors.push(connector);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Environment name must not be empty".into());
        }
        self.observation_space
            .validate()
            .map_err(|e| format!("observation_space: {e}"))?;
        self.action_space
            .validate()
            .map_err(|e| format!("action_space: {e}"))?;
        if let Some(ref backend) = self.sim_backend {
            backend.validate().map_err(|e| format!("sim_backend: {e}"))?;
        }
        for conn in &self.connectors {
            conn.validate().map_err(|e| format!("connector '{}': {e}", conn.name))?;
        }
        if let Some(ref rand) = self.randomization {
            rand.validate().map_err(|e| format!("randomization: {e}"))?;
        }
        Ok(())
    }

    /// Compute total reward given state, action, and returns.
    pub fn compute_reward(&self, state: &[f64], action: &[f64], returns: &[f64]) -> f64 {
        self.reward_components
            .iter()
            .map(|c| c.compute(state, action, returns))
            .sum()
    }

    /// Check all safety constraints, returning violations.
    pub fn check_safety(&mut self, state: &[f64], action: &[f64]) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();
        for constraint in &mut self.safety_constraints {
            if let Some(v) = constraint.check(state, action) {
                violations.push(v);
            }
        }
        violations
    }

    /// Serialize to a YAML-like string.
    pub fn to_yaml(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str(&format!("name: {}\n", self.name));
        out.push_str(&format!("version: {}\n", self.version));
        out.push_str(&format!("description: {}\n", self.description));
        out.push_str(&format!("observation_space: {}\n", self.observation_space));
        out.push_str(&format!("action_space: {}\n", self.action_space));
        out.push_str(&format!("training_mode: {}\n", self.training_mode));
        out.push_str(&format!("max_episode_steps: {}\n", self.max_episode_steps));
        out.push_str(&format!("reward_components: {}\n", self.reward_components.len()));
        out.push_str(&format!("safety_constraints: {}\n", self.safety_constraints.len()));
        out.push_str(&format!("connectors: {}\n", self.connectors.len()));
        if !self.tags.is_empty() {
            out.push_str(&format!("tags: [{}]\n", self.tags.join(", ")));
        }
        out
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §10  Environment Definition DSL Parser
// ══════════════════════════════════════════════════════════════════════════════

/// Parse a YAML-like environment definition string.
pub struct EnvDefParser;

impl EnvDefParser {
    /// Parse a key: value line.
    fn parse_kv(line: &str) -> Option<(&str, &str)> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let idx = line.find(':')?;
        let key = line[..idx].trim();
        let val = line[idx + 1..].trim();
        Some((key, val))
    }

    /// Parse a space type from a string descriptor.
    pub fn parse_space(desc: &str) -> Result<SpaceType, String> {
        let desc = desc.trim();
        if desc.starts_with("Box(") && desc.ends_with(')') {
            let inner = &desc[4..desc.len() - 1];
            // Format: "shape=[3], low=-1.0, high=1.0, dtype=float32"
            let mut shape = vec![1usize];
            let mut dtype = DType::Float32;
            let mut low_val = -1.0f64;
            let mut high_val = 1.0f64;

            for part in inner.split(',') {
                let part = part.trim();
                if let Some(s) = part.strip_prefix("shape=") {
                    let s = s.trim_start_matches('[').trim_end_matches(']');
                    shape = s
                        .split('x')
                        .filter(|v| !v.is_empty())
                        .map(|v| v.trim().parse::<usize>().unwrap_or(1))
                        .collect();
                } else if let Some(s) = part.strip_prefix("low=") {
                    low_val = s.parse().unwrap_or(-1.0);
                } else if let Some(s) = part.strip_prefix("high=") {
                    high_val = s.parse().unwrap_or(1.0);
                } else if let Some(s) = part.strip_prefix("dtype=") {
                    dtype = match s.trim() {
                        "float64" => DType::Float64,
                        "int32" => DType::Int32,
                        "int64" => DType::Int64,
                        "bool" => DType::Bool,
                        "uint8" => DType::UInt8,
                        _ => DType::Float32,
                    };
                }
            }
            let size: usize = shape.iter().product::<usize>().max(1);
            Ok(SpaceType::Box {
                low: vec![low_val; size],
                high: vec![high_val; size],
                shape,
                dtype,
            })
        } else if desc.starts_with("Discrete(") && desc.ends_with(')') {
            let n: u64 = desc[9..desc.len() - 1]
                .trim()
                .parse()
                .map_err(|_| "Invalid Discrete n".to_string())?;
            Ok(SpaceType::Discrete { n })
        } else if desc.starts_with("MultiBinary(") && desc.ends_with(')') {
            let n: usize = desc[12..desc.len() - 1]
                .trim()
                .parse()
                .map_err(|_| "Invalid MultiBinary n".to_string())?;
            Ok(SpaceType::MultiBinary { n })
        } else if desc.starts_with("MultiDiscrete(") && desc.ends_with(')') {
            let inner = &desc[14..desc.len() - 1];
            let inner = inner.trim_start_matches('[').trim_end_matches(']');
            let nvec: Result<Vec<u64>, _> = inner.split(',').map(|v| v.trim().parse()).collect();
            let nvec = nvec.map_err(|_| "Invalid MultiDiscrete nvec".to_string())?;
            Ok(SpaceType::MultiDiscrete { nvec })
        } else {
            Err(format!("Unknown space type: {desc}"))
        }
    }

    /// Parse a full environment definition from YAML-like text.
    pub fn parse(input: &str) -> Result<EnvironmentDef, String> {
        let mut name = String::new();
        let mut version = String::new();
        let mut description = String::new();
        let mut obs_space_str = String::new();
        let mut act_space_str = String::new();
        let mut max_steps: u64 = 1000;
        let mut training_mode = TrainingMode::Simulation;
        let mut tags = Vec::new();

        for line in input.lines() {
            if let Some((key, val)) = Self::parse_kv(line) {
                match key {
                    "name" => name = val.to_string(),
                    "version" => version = val.to_string(),
                    "description" => description = val.to_string(),
                    "observation_space" => obs_space_str = val.to_string(),
                    "action_space" => act_space_str = val.to_string(),
                    "max_episode_steps" => {
                        max_steps = val.parse().unwrap_or(1000);
                    }
                    "training_mode" => {
                        training_mode = match val {
                            "RealWorld" => TrainingMode::RealWorld,
                            "HybridSimReal" => TrainingMode::HybridSimReal,
                            _ => TrainingMode::Simulation,
                        };
                    }
                    "tags" => {
                        let inner = val.trim_start_matches('[').trim_end_matches(']');
                        tags = inner.split(',').map(|t| t.trim().to_string()).collect();
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err("Missing required field: name".into());
        }

        let obs_space = if obs_space_str.is_empty() {
            SpaceType::Box {
                low: vec![-1.0],
                high: vec![1.0],
                shape: vec![1],
                dtype: DType::Float32,
            }
        } else {
            Self::parse_space(&obs_space_str)?
        };

        let act_space = if act_space_str.is_empty() {
            SpaceType::Discrete { n: 2 }
        } else {
            Self::parse_space(&act_space_str)?
        };

        let mut env = EnvironmentDef::new(&name, obs_space, act_space);
        env.version = if version.is_empty() {
            "0.1.0".to_string()
        } else {
            version
        };
        env.description = description;
        env.max_episode_steps = max_steps;
        env.training_mode = training_mode;
        env.tags = tags;
        Ok(env)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §11  Environment Registry
// ══════════════════════════════════════════════════════════════════════════════

/// Entry in the environment registry.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryEntry {
    pub env_def: EnvironmentDef,
    pub registered_at: u64,
    pub author: String,
    pub downloads: u64,
    pub rating: f64,
}

/// Environment registry for discovery and management.
#[derive(Debug, Clone)]
pub struct EnvironmentRegistry {
    pub entries: HashMap<String, RegistryEntry>,
}

impl EnvironmentRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        env_def: EnvironmentDef,
        author: &str,
        timestamp: u64,
    ) -> Result<(), String> {
        if self.entries.contains_key(&env_def.name) {
            return Err(format!("Environment '{}' already registered", env_def.name));
        }
        env_def.validate()?;
        let name = env_def.name.clone();
        self.entries.insert(
            name,
            RegistryEntry {
                env_def,
                registered_at: timestamp,
                author: author.to_string(),
                downloads: 0,
                rating: 0.0,
            },
        );
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> Result<(), String> {
        self.entries
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| format!("Environment '{name}' not found"))
    }

    pub fn get(&self, name: &str) -> Option<&RegistryEntry> {
        self.entries.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.keys().map(|k| k.as_str()).collect();
        names.sort();
        names
    }

    /// Search by tag or keyword in name/description.
    pub fn search(&self, query: &str) -> Vec<&RegistryEntry> {
        let q = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| {
                e.env_def.name.to_lowercase().contains(&q)
                    || e.env_def.description.to_lowercase().contains(&q)
                    || e.env_def.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Search by training mode.
    pub fn search_by_mode(&self, mode: &TrainingMode) -> Vec<&RegistryEntry> {
        self.entries
            .values()
            .filter(|e| &e.env_def.training_mode == mode)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn record_download(&mut self, name: &str) -> Result<(), String> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| format!("Environment '{name}' not found"))?;
        entry.downloads += 1;
        Ok(())
    }

    pub fn set_rating(&mut self, name: &str, rating: f64) -> Result<(), String> {
        if !(0.0..=5.0).contains(&rating) {
            return Err("Rating must be between 0 and 5".into());
        }
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| format!("Environment '{name}' not found"))?;
        entry.rating = rating;
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §12  Vectorized Environments
// ══════════════════════════════════════════════════════════════════════════════

/// Stepping mode for vectorized environments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VecEnvMode {
    Sync,
    Async,
}

impl VecEnvMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sync => "Sync",
            Self::Async => "Async",
        }
    }
}

impl std::fmt::Display for VecEnvMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Result of stepping a single sub-environment.
#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    pub observation: Vec<f64>,
    pub reward: f64,
    pub done: bool,
    pub truncated: bool,
    pub info: HashMap<String, String>,
}

impl StepResult {
    pub fn new(observation: Vec<f64>, reward: f64, done: bool, truncated: bool) -> Self {
        Self {
            observation,
            reward,
            done,
            truncated,
            info: HashMap::new(),
        }
    }
}

/// A single sub-environment within a vectorized env.
#[derive(Debug, Clone)]
pub struct SubEnvironment {
    pub index: usize,
    pub state: Vec<f64>,
    pub step_count: u64,
    pub episode_reward: f64,
    pub done: bool,
}

impl SubEnvironment {
    pub fn new(index: usize, obs_size: usize) -> Self {
        Self {
            index,
            state: vec![0.0; obs_size],
            step_count: 0,
            episode_reward: 0.0,
            done: false,
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.state {
            *v = 0.0;
        }
        self.step_count = 0;
        self.episode_reward = 0.0;
        self.done = false;
    }

    /// Simple step: apply action as delta, compute reward.
    pub fn step(&mut self, action: &[f64], max_steps: u64) -> StepResult {
        for (s, a) in self.state.iter_mut().zip(action.iter()) {
            *s += a;
        }
        self.step_count += 1;
        let reward = -self.state.iter().map(|s| s.powi(2)).sum::<f64>().sqrt();
        self.episode_reward += reward;
        let truncated = self.step_count >= max_steps;
        let dist = self.state.iter().map(|s| s.powi(2)).sum::<f64>().sqrt();
        self.done = dist < 0.01 || truncated;

        StepResult::new(self.state.clone(), reward, self.done, truncated)
    }
}

/// Vectorized environment manager.
#[derive(Debug, Clone)]
pub struct VecEnv {
    pub envs: Vec<SubEnvironment>,
    pub mode: VecEnvMode,
    pub obs_size: usize,
    pub max_episode_steps: u64,
    pub total_steps: u64,
    pub total_episodes: u64,
}

impl VecEnv {
    pub fn new(num_envs: usize, obs_size: usize, mode: VecEnvMode) -> Self {
        let envs = (0..num_envs)
            .map(|i| SubEnvironment::new(i, obs_size))
            .collect();
        Self {
            envs,
            mode,
            obs_size,
            max_episode_steps: 1000,
            total_steps: 0,
            total_episodes: 0,
        }
    }

    pub fn num_envs(&self) -> usize {
        self.envs.len()
    }

    pub fn reset_all(&mut self) {
        for env in &mut self.envs {
            env.reset();
        }
    }

    /// Step all environments with the given batch of actions.
    pub fn step_all(&mut self, actions: &[Vec<f64>]) -> Result<Vec<StepResult>, String> {
        if actions.len() != self.envs.len() {
            return Err(format!(
                "Expected {} actions, got {}",
                self.envs.len(),
                actions.len()
            ));
        }
        let mut results = Vec::with_capacity(self.envs.len());
        for (env, action) in self.envs.iter_mut().zip(actions.iter()) {
            let result = env.step(action, self.max_episode_steps);
            if result.done {
                // Auto-reset
                env.reset();
                // Track completed episodes but don't increment total_episodes here
                // (caller handles it via check)
            }
            results.push(result);
        }
        self.total_steps += self.envs.len() as u64;
        let done_count = results.iter().filter(|r| r.done).count() as u64;
        self.total_episodes += done_count;
        Ok(results)
    }

    /// Get current observations from all sub-environments.
    pub fn observations(&self) -> Vec<Vec<f64>> {
        self.envs.iter().map(|e| e.state.clone()).collect()
    }

    /// Average episode reward across completed episodes (approximate).
    pub fn avg_episode_reward(&self) -> f64 {
        if self.envs.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.envs.iter().map(|e| e.episode_reward).sum();
        sum / self.envs.len() as f64
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §13  Gymnasium / PettingZoo Compatibility
// ══════════════════════════════════════════════════════════════════════════════

/// Gymnasium-compatible environment wrapper trait (single-agent).
pub trait GymnasiumEnv {
    fn reset(&mut self, seed: Option<u64>) -> (Vec<f64>, HashMap<String, String>);
    fn step(&mut self, action: &[f64]) -> StepResult;
    fn observation_space(&self) -> &SpaceType;
    fn action_space(&self) -> &SpaceType;
    fn render(&self) -> String;
    fn close(&mut self);
}

/// PettingZoo-compatible multi-agent trait.
pub trait PettingZooEnv {
    fn agents(&self) -> Vec<String>;
    fn reset_multi(&mut self, seed: Option<u64>) -> HashMap<String, Vec<f64>>;
    fn step_agent(&mut self, agent: &str, action: &[f64]) -> StepResult;
    fn observation_space_for(&self, agent: &str) -> Option<&SpaceType>;
    fn action_space_for(&self, agent: &str) -> Option<&SpaceType>;
    fn agent_selection(&self) -> Option<String>;
}

/// A concrete single-agent environment implementing the Gymnasium trait.
#[derive(Debug, Clone)]
pub struct SimpleGymEnv {
    pub env_def: EnvironmentDef,
    pub state: Vec<f64>,
    pub step_count: u64,
    pub done: bool,
    pub seed: u64,
}

impl SimpleGymEnv {
    pub fn new(env_def: EnvironmentDef) -> Self {
        let obs_size = env_def.observation_space.flat_size();
        Self {
            env_def,
            state: vec![0.0; obs_size],
            step_count: 0,
            done: false,
            seed: 0,
        }
    }
}

impl GymnasiumEnv for SimpleGymEnv {
    fn reset(&mut self, seed: Option<u64>) -> (Vec<f64>, HashMap<String, String>) {
        self.seed = seed.unwrap_or(42);
        let size = self.env_def.observation_space.flat_size();
        self.state = vec![0.0; size];
        self.step_count = 0;
        self.done = false;
        (self.state.clone(), HashMap::new())
    }

    fn step(&mut self, action: &[f64]) -> StepResult {
        for (s, a) in self.state.iter_mut().zip(action.iter()) {
            *s += a;
        }
        self.step_count += 1;
        let reward = self.env_def.compute_reward(&self.state, action, &[]);
        let truncated = self.step_count >= self.env_def.max_episode_steps;
        self.done = truncated;
        StepResult::new(self.state.clone(), reward, self.done, truncated)
    }

    fn observation_space(&self) -> &SpaceType {
        &self.env_def.observation_space
    }

    fn action_space(&self) -> &SpaceType {
        &self.env_def.action_space
    }

    fn render(&self) -> String {
        format!(
            "Step {}: state={:?}, done={}",
            self.step_count, self.state, self.done
        )
    }

    fn close(&mut self) {
        self.done = true;
    }
}

/// Multi-agent environment implementing PettingZoo trait.
#[derive(Debug, Clone)]
pub struct MultiAgentEnv {
    pub env_def: EnvironmentDef,
    pub agent_names: Vec<String>,
    pub agent_states: HashMap<String, Vec<f64>>,
    pub agent_obs_spaces: HashMap<String, SpaceType>,
    pub agent_act_spaces: HashMap<String, SpaceType>,
    pub current_agent_idx: usize,
    pub step_count: u64,
}

impl MultiAgentEnv {
    pub fn new(env_def: EnvironmentDef, agent_names: Vec<String>) -> Self {
        let obs_size = env_def.observation_space.flat_size();
        let mut agent_states = HashMap::new();
        let mut agent_obs_spaces = HashMap::new();
        let mut agent_act_spaces = HashMap::new();
        for name in &agent_names {
            agent_states.insert(name.clone(), vec![0.0; obs_size]);
            agent_obs_spaces.insert(name.clone(), env_def.observation_space.clone());
            agent_act_spaces.insert(name.clone(), env_def.action_space.clone());
        }
        Self {
            env_def,
            agent_names,
            agent_states,
            agent_obs_spaces,
            agent_act_spaces,
            current_agent_idx: 0,
            step_count: 0,
        }
    }
}

impl PettingZooEnv for MultiAgentEnv {
    fn agents(&self) -> Vec<String> {
        self.agent_names.clone()
    }

    fn reset_multi(&mut self, _seed: Option<u64>) -> HashMap<String, Vec<f64>> {
        let obs_size = self.env_def.observation_space.flat_size();
        for state in self.agent_states.values_mut() {
            *state = vec![0.0; obs_size];
        }
        self.current_agent_idx = 0;
        self.step_count = 0;
        self.agent_states.clone()
    }

    fn step_agent(&mut self, agent: &str, action: &[f64]) -> StepResult {
        if let Some(state) = self.agent_states.get_mut(agent) {
            for (s, a) in state.iter_mut().zip(action.iter()) {
                *s += a;
            }
            self.step_count += 1;
            self.current_agent_idx = (self.current_agent_idx + 1) % self.agent_names.len();
            let reward = -state.iter().map(|s| s.powi(2)).sum::<f64>().sqrt();
            let done = self.step_count >= self.env_def.max_episode_steps;
            StepResult::new(state.clone(), reward, done, done)
        } else {
            StepResult::new(vec![], 0.0, true, false)
        }
    }

    fn observation_space_for(&self, agent: &str) -> Option<&SpaceType> {
        self.agent_obs_spaces.get(agent)
    }

    fn action_space_for(&self, agent: &str) -> Option<&SpaceType> {
        self.agent_act_spaces.get(agent)
    }

    fn agent_selection(&self) -> Option<String> {
        self.agent_names.get(self.current_agent_idx).cloned()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §14  Hybrid Sim+Real Pipeline
// ══════════════════════════════════════════════════════════════════════════════

/// Stage in the hybrid sim-to-real pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    SimTraining,
    DomainRandomization,
    SimEvaluation,
    RealWorldCalibration,
    RealWorldFinetuning,
    Deployment,
}

impl PipelineStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SimTraining => "SimTraining",
            Self::DomainRandomization => "DomainRandomization",
            Self::SimEvaluation => "SimEvaluation",
            Self::RealWorldCalibration => "RealWorldCalibration",
            Self::RealWorldFinetuning => "RealWorldFinetuning",
            Self::Deployment => "Deployment",
        }
    }

    pub fn next(&self) -> Option<PipelineStage> {
        match self {
            Self::SimTraining => Some(Self::DomainRandomization),
            Self::DomainRandomization => Some(Self::SimEvaluation),
            Self::SimEvaluation => Some(Self::RealWorldCalibration),
            Self::RealWorldCalibration => Some(Self::RealWorldFinetuning),
            Self::RealWorldFinetuning => Some(Self::Deployment),
            Self::Deployment => None,
        }
    }
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Domain adaptation metric tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptationMetrics {
    pub sim_performance: f64,
    pub real_performance: f64,
    pub transfer_gap: f64,
    pub adaptation_iterations: u32,
    pub convergence_threshold: f64,
}

impl AdaptationMetrics {
    pub fn new(convergence_threshold: f64) -> Self {
        Self {
            sim_performance: 0.0,
            real_performance: 0.0,
            transfer_gap: f64::MAX,
            adaptation_iterations: 0,
            convergence_threshold,
        }
    }

    pub fn update(&mut self, sim_perf: f64, real_perf: f64) {
        self.sim_performance = sim_perf;
        self.real_performance = real_perf;
        self.transfer_gap = (sim_perf - real_perf).abs();
        self.adaptation_iterations += 1;
    }

    pub fn is_converged(&self) -> bool {
        self.transfer_gap < self.convergence_threshold
    }

    pub fn transfer_efficiency(&self) -> f64 {
        if self.sim_performance.abs() < 1e-10 {
            return 0.0;
        }
        self.real_performance / self.sim_performance
    }
}

/// Hybrid sim-to-real training pipeline.
#[derive(Debug, Clone)]
pub struct HybridPipeline {
    pub current_stage: PipelineStage,
    pub metrics: AdaptationMetrics,
    pub stage_history: Vec<(PipelineStage, f64)>, // (stage, timestamp)
    pub sim_episodes: u64,
    pub real_episodes: u64,
    pub config: HashMap<String, String>,
}

impl HybridPipeline {
    pub fn new(convergence_threshold: f64) -> Self {
        Self {
            current_stage: PipelineStage::SimTraining,
            metrics: AdaptationMetrics::new(convergence_threshold),
            stage_history: Vec::new(),
            sim_episodes: 0,
            real_episodes: 0,
            config: HashMap::new(),
        }
    }

    pub fn advance_stage(&mut self, timestamp: f64) -> Result<PipelineStage, String> {
        let next = self
            .current_stage
            .next()
            .ok_or("Already at final stage (Deployment)")?;
        self.stage_history
            .push((self.current_stage.clone(), timestamp));
        self.current_stage = next.clone();
        Ok(next)
    }

    pub fn record_sim_episode(&mut self, reward: f64) {
        self.sim_episodes += 1;
        // Running average
        let n = self.sim_episodes as f64;
        self.metrics.sim_performance =
            self.metrics.sim_performance * ((n - 1.0) / n) + reward / n;
    }

    pub fn record_real_episode(&mut self, reward: f64) {
        self.real_episodes += 1;
        let n = self.real_episodes as f64;
        self.metrics.real_performance =
            self.metrics.real_performance * ((n - 1.0) / n) + reward / n;
        self.metrics.transfer_gap =
            (self.metrics.sim_performance - self.metrics.real_performance).abs();
        self.metrics.adaptation_iterations += 1;
    }

    pub fn is_ready_for_deployment(&self) -> bool {
        self.metrics.is_converged()
            && self.current_stage == PipelineStage::RealWorldFinetuning
    }

    pub fn summary(&self) -> String {
        format!(
            "Stage: {}, Sim eps: {}, Real eps: {}, Gap: {:.4}, Efficiency: {:.2}%",
            self.current_stage,
            self.sim_episodes,
            self.real_episodes,
            self.metrics.transfer_gap,
            self.metrics.transfer_efficiency() * 100.0,
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §15  Finance-Specific
// ══════════════════════════════════════════════════════════════════════════════

/// Order side in a financial market.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "Buy"),
            Self::Sell => write!(f, "Sell"),
        }
    }
}

/// Order type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Market => write!(f, "Market"),
            Self::Limit => write!(f, "Limit"),
            Self::Stop => write!(f, "Stop"),
            Self::StopLimit => write!(f, "StopLimit"),
        }
    }
}

/// A financial order.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: u64,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub filled: bool,
    pub fill_price: Option<f64>,
}

impl Order {
    pub fn market(id: u64, symbol: &str, side: OrderSide, qty: f64) -> Self {
        Self {
            id,
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity: qty,
            price: None,
            stop_price: None,
            filled: false,
            fill_price: None,
        }
    }

    pub fn limit(id: u64, symbol: &str, side: OrderSide, qty: f64, price: f64) -> Self {
        Self {
            id,
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            quantity: qty,
            price: Some(price),
            stop_price: None,
            filled: false,
            fill_price: None,
        }
    }
}

/// Level in an order book.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: f64,
    pub order_count: u32,
}

/// Simulated order book.
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub last_trade_price: f64,
    pub volume_24h: f64,
}

impl OrderBook {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bids: Vec::new(),
            asks: Vec::new(),
            last_trade_price: 0.0,
            volume_24h: 0.0,
        }
    }

    pub fn add_bid(&mut self, price: f64, quantity: f64) {
        self.bids.push(OrderBookLevel {
            price,
            quantity,
            order_count: 1,
        });
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn add_ask(&mut self, price: f64, quantity: f64) {
        self.asks.push(OrderBookLevel {
            price,
            quantity,
            order_count: 1,
        });
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|l| l.price)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|l| l.price)
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    /// Try to fill a market order.
    pub fn fill_market_order(&mut self, order: &mut Order) -> bool {
        let levels = match order.side {
            OrderSide::Buy => &mut self.asks,
            OrderSide::Sell => &mut self.bids,
        };
        if levels.is_empty() {
            return false;
        }
        let fill_price = levels[0].price;
        if levels[0].quantity >= order.quantity {
            levels[0].quantity -= order.quantity;
            if levels[0].quantity < 1e-10 {
                levels.remove(0);
            }
        } else {
            return false; // Partial fills not supported in this simple model
        }
        order.filled = true;
        order.fill_price = Some(fill_price);
        self.last_trade_price = fill_price;
        self.volume_24h += order.quantity * fill_price;
        true
    }

    pub fn total_bid_depth(&self) -> f64 {
        self.bids.iter().map(|l| l.quantity).sum()
    }

    pub fn total_ask_depth(&self) -> f64 {
        self.asks.iter().map(|l| l.quantity).sum()
    }
}

/// Financial market regulatory constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct FinanceRegConstraint {
    pub name: String,
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_leverage: f64,
    pub restricted_symbols: Vec<String>,
    pub trading_hours_start: u32, // minutes from midnight
    pub trading_hours_end: u32,
}

impl FinanceRegConstraint {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            max_position_size: 1_000_000.0,
            max_daily_loss: 50_000.0,
            max_leverage: 10.0,
            restricted_symbols: Vec::new(),
            trading_hours_start: 570,  // 9:30 AM
            trading_hours_end: 960,    // 4:00 PM
        }
    }

    pub fn check_order(&self, order: &Order, current_position: f64) -> Result<(), String> {
        let new_position = match order.side {
            OrderSide::Buy => current_position + order.quantity,
            OrderSide::Sell => current_position - order.quantity,
        };
        if new_position.abs() > self.max_position_size {
            return Err(format!(
                "Position size {:.2} exceeds limit {:.2}",
                new_position.abs(),
                self.max_position_size
            ));
        }
        if self.restricted_symbols.contains(&order.symbol) {
            return Err(format!("Symbol '{}' is restricted", order.symbol));
        }
        Ok(())
    }

    pub fn is_within_trading_hours(&self, minutes_from_midnight: u32) -> bool {
        minutes_from_midnight >= self.trading_hours_start
            && minutes_from_midnight <= self.trading_hours_end
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §16  Robotics-Specific
// ══════════════════════════════════════════════════════════════════════════════

/// Joint type for robotic systems.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JointType {
    Revolute,
    Prismatic,
    Fixed,
    Continuous,
    Planar,
    Floating,
}

impl JointType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Revolute => "Revolute",
            Self::Prismatic => "Prismatic",
            Self::Fixed => "Fixed",
            Self::Continuous => "Continuous",
            Self::Planar => "Planar",
            Self::Floating => "Floating",
        }
    }

    pub fn has_limits(&self) -> bool {
        matches!(self, Self::Revolute | Self::Prismatic)
    }
}

impl std::fmt::Display for JointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A robot joint with limits.
#[derive(Debug, Clone, PartialEq)]
pub struct RobotJoint {
    pub name: String,
    pub joint_type: JointType,
    pub position_min: f64,
    pub position_max: f64,
    pub velocity_max: f64,
    pub torque_max: f64,
    pub current_position: f64,
    pub current_velocity: f64,
}

impl RobotJoint {
    pub fn new(name: &str, joint_type: JointType) -> Self {
        let (pos_min, pos_max) = match &joint_type {
            JointType::Revolute => (-std::f64::consts::PI, std::f64::consts::PI),
            JointType::Prismatic => (-1.0, 1.0),
            _ => (-f64::MAX, f64::MAX),
        };
        Self {
            name: name.to_string(),
            joint_type,
            position_min: pos_min,
            position_max: pos_max,
            velocity_max: 10.0,
            torque_max: 100.0,
            current_position: 0.0,
            current_velocity: 0.0,
        }
    }

    pub fn with_limits(mut self, pos_min: f64, pos_max: f64, vel_max: f64, torque_max: f64) -> Self {
        self.position_min = pos_min;
        self.position_max = pos_max;
        self.velocity_max = vel_max;
        self.torque_max = torque_max;
        self
    }

    /// Check if current state is within limits.
    pub fn is_within_limits(&self) -> bool {
        if !self.joint_type.has_limits() {
            return true;
        }
        self.current_position >= self.position_min
            && self.current_position <= self.position_max
            && self.current_velocity.abs() <= self.velocity_max
    }

    /// Apply a torque command, returning whether it was clamped.
    pub fn apply_torque(&mut self, torque: f64, dt: f64) -> bool {
        let clamped_torque = torque.clamp(-self.torque_max, self.torque_max);
        let was_clamped = (clamped_torque - torque).abs() > 1e-10;

        // Simple dynamics: acceleration = torque (unit mass)
        self.current_velocity += clamped_torque * dt;
        self.current_velocity = self
            .current_velocity
            .clamp(-self.velocity_max, self.velocity_max);
        self.current_position += self.current_velocity * dt;

        if self.joint_type.has_limits() {
            self.current_position = self
                .current_position
                .clamp(self.position_min, self.position_max);
        }

        was_clamped
    }

    pub fn reset(&mut self) {
        self.current_position = 0.0;
        self.current_velocity = 0.0;
    }
}

/// Sim-to-real transfer status for a robotic system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimToRealStatus {
    SimOnly,
    CalibrationPending,
    Calibrated,
    RealValidated,
    Deployed,
}

impl SimToRealStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SimOnly => "SimOnly",
            Self::CalibrationPending => "CalibrationPending",
            Self::Calibrated => "Calibrated",
            Self::RealValidated => "RealValidated",
            Self::Deployed => "Deployed",
        }
    }
}

impl std::fmt::Display for SimToRealStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// ROS 2 bridge topic descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct Ros2Topic {
    pub name: String,
    pub msg_type: String,
    pub direction: Ros2Direction,
    pub qos_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ros2Direction {
    Subscribe,
    Publish,
    Service,
    Action,
}

impl Ros2Direction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Subscribe => "Subscribe",
            Self::Publish => "Publish",
            Self::Service => "Service",
            Self::Action => "Action",
        }
    }
}

impl std::fmt::Display for Ros2Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// ROS 2 bridge configuration.
#[derive(Debug, Clone)]
pub struct Ros2Bridge {
    pub node_name: String,
    pub namespace: String,
    pub topics: Vec<Ros2Topic>,
    pub connected: bool,
    pub message_count: u64,
}

impl Ros2Bridge {
    pub fn new(node_name: &str, namespace: &str) -> Self {
        Self {
            node_name: node_name.to_string(),
            namespace: namespace.to_string(),
            topics: Vec::new(),
            connected: false,
            message_count: 0,
        }
    }

    pub fn add_topic(&mut self, name: &str, msg_type: &str, direction: Ros2Direction) {
        self.topics.push(Ros2Topic {
            name: name.to_string(),
            msg_type: msg_type.to_string(),
            direction,
            qos_depth: 10,
        });
    }

    pub fn subscribe_topics(&self) -> Vec<&Ros2Topic> {
        self.topics
            .iter()
            .filter(|t| t.direction == Ros2Direction::Subscribe)
            .collect()
    }

    pub fn publish_topics(&self) -> Vec<&Ros2Topic> {
        self.topics
            .iter()
            .filter(|t| t.direction == Ros2Direction::Publish)
            .collect()
    }

    pub fn connect(&mut self) -> Result<(), String> {
        if self.topics.is_empty() {
            return Err("No topics configured".into());
        }
        self.connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn record_message(&mut self) {
        self.message_count += 1;
    }
}

/// Robotics sim-to-real pipeline manager.
#[derive(Debug, Clone)]
pub struct RoboticsSimToReal {
    pub joints: Vec<RobotJoint>,
    pub status: SimToRealStatus,
    pub ros2_bridge: Option<Ros2Bridge>,
    pub calibration_error: f64,
    pub real_world_episodes: u64,
    pub sim_episodes: u64,
}

impl RoboticsSimToReal {
    pub fn new() -> Self {
        Self {
            joints: Vec::new(),
            status: SimToRealStatus::SimOnly,
            ros2_bridge: None,
            calibration_error: f64::MAX,
            real_world_episodes: 0,
            sim_episodes: 0,
        }
    }

    pub fn add_joint(&mut self, joint: RobotJoint) {
        self.joints.push(joint);
    }

    pub fn set_ros2_bridge(&mut self, bridge: Ros2Bridge) {
        self.ros2_bridge = Some(bridge);
    }

    pub fn all_joints_within_limits(&self) -> bool {
        self.joints.iter().all(|j| j.is_within_limits())
    }

    /// Apply torques to all joints. Returns indices of clamped joints.
    pub fn apply_torques(&mut self, torques: &[f64], dt: f64) -> Vec<usize> {
        let mut clamped = Vec::new();
        for (i, (joint, torque)) in self.joints.iter_mut().zip(torques.iter()).enumerate() {
            if joint.apply_torque(*torque, dt) {
                clamped.push(i);
            }
        }
        clamped
    }

    pub fn calibrate(&mut self, error: f64) {
        self.calibration_error = error;
        if error < 0.01 {
            self.status = SimToRealStatus::Calibrated;
        } else {
            self.status = SimToRealStatus::CalibrationPending;
        }
    }

    pub fn validate_real(&mut self) -> bool {
        if self.status == SimToRealStatus::Calibrated && self.calibration_error < 0.01 {
            self.status = SimToRealStatus::RealValidated;
            true
        } else {
            false
        }
    }

    pub fn deploy(&mut self) -> Result<(), String> {
        if self.status != SimToRealStatus::RealValidated {
            return Err(format!(
                "Cannot deploy from status '{}', need RealValidated",
                self.status
            ));
        }
        self.status = SimToRealStatus::Deployed;
        Ok(())
    }

    pub fn joint_positions(&self) -> Vec<f64> {
        self.joints.iter().map(|j| j.current_position).collect()
    }

    pub fn joint_velocities(&self) -> Vec<f64> {
        self.joints.iter().map(|j| j.current_velocity).collect()
    }

    pub fn reset_all_joints(&mut self) {
        for joint in &mut self.joints {
            joint.reset();
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// §17  Top-Level RL-OS Orchestrator
// ══════════════════════════════════════════════════════════════════════════════

/// The main RL-OS controller tying everything together.
#[derive(Debug, Clone)]
pub struct RlEnvOs {
    pub registry: EnvironmentRegistry,
    pub version_control: EnvVersionControl,
    pub replay_manager: ReplayManager,
    pub pipelines: HashMap<String, HybridPipeline>,
    pub robotics: HashMap<String, RoboticsSimToReal>,
    pub order_books: HashMap<String, OrderBook>,
    pub global_step: u64,
}

impl RlEnvOs {
    pub fn new() -> Self {
        Self {
            registry: EnvironmentRegistry::new(),
            version_control: EnvVersionControl::new(),
            replay_manager: ReplayManager::new(1000),
            pipelines: HashMap::new(),
            robotics: HashMap::new(),
            order_books: HashMap::new(),
            global_step: 0,
        }
    }

    pub fn register_env(
        &mut self,
        env_def: EnvironmentDef,
        author: &str,
        timestamp: u64,
    ) -> Result<(), String> {
        let yaml = env_def.to_yaml();
        self.registry.register(env_def, author, timestamp)?;
        self.version_control.commit("Initial registration", &yaml, timestamp);
        Ok(())
    }

    pub fn create_pipeline(&mut self, name: &str, convergence: f64) {
        self.pipelines
            .insert(name.to_string(), HybridPipeline::new(convergence));
    }

    pub fn create_robotics(&mut self, name: &str) {
        self.robotics
            .insert(name.to_string(), RoboticsSimToReal::new());
    }

    pub fn create_order_book(&mut self, symbol: &str) {
        self.order_books
            .insert(symbol.to_string(), OrderBook::new(symbol));
    }

    pub fn summary(&self) -> String {
        format!(
            "RL-OS: {} envs, {} commits, {} trajectories, {} pipelines, {} robots, {} books",
            self.registry.count(),
            self.version_control.commit_count(),
            self.replay_manager.trajectories.len(),
            self.pipelines.len(),
            self.robotics.len(),
            self.order_books.len(),
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Distribution tests ────────────────────────────────────────────

    #[test]
    fn test_distribution_uniform_validate() {
        let d = Distribution::Uniform { low: 0.0, high: 1.0 };
        assert!(d.validate().is_ok());
    }

    #[test]
    fn test_distribution_uniform_invalid() {
        let d = Distribution::Uniform { low: 5.0, high: 2.0 };
        assert!(d.validate().is_err());
    }

    #[test]
    fn test_distribution_normal_validate() {
        let d = Distribution::Normal { mean: 0.0, std_dev: 1.0 };
        assert!(d.validate().is_ok());
    }

    #[test]
    fn test_distribution_normal_invalid_std() {
        let d = Distribution::Normal { mean: 0.0, std_dev: -1.0 };
        assert!(d.validate().is_err());
    }

    #[test]
    fn test_distribution_lognormal_validate() {
        let d = Distribution::LogNormal { mu: 0.0, sigma: 0.5 };
        assert!(d.validate().is_ok());
    }

    #[test]
    fn test_distribution_lognormal_invalid() {
        let d = Distribution::LogNormal { mu: 0.0, sigma: 0.0 };
        assert!(d.validate().is_err());
    }

    #[test]
    fn test_distribution_categorical_validate() {
        let d = Distribution::Categorical { probs: vec![0.3, 0.7] };
        assert!(d.validate().is_ok());
    }

    #[test]
    fn test_distribution_categorical_bad_sum() {
        let d = Distribution::Categorical { probs: vec![0.3, 0.3] };
        assert!(d.validate().is_err());
    }

    #[test]
    fn test_distribution_categorical_empty() {
        let d = Distribution::Categorical { probs: vec![] };
        assert!(d.validate().is_err());
    }

    #[test]
    fn test_distribution_sample_deterministic() {
        let d = Distribution::Uniform { low: 0.0, high: 10.0 };
        let v1 = d.sample_deterministic(42);
        let v2 = d.sample_deterministic(42);
        assert_eq!(v1, v2); // deterministic
    }

    #[test]
    fn test_distribution_labels() {
        assert_eq!(Distribution::Uniform { low: 0.0, high: 1.0 }.label(), "Uniform");
        assert_eq!(Distribution::Normal { mean: 0.0, std_dev: 1.0 }.label(), "Normal");
        assert_eq!(Distribution::LogNormal { mu: 0.0, sigma: 1.0 }.label(), "LogNormal");
    }

    #[test]
    fn test_distribution_display() {
        let d = Distribution::Uniform { low: -1.0, high: 1.0 };
        assert!(format!("{d}").contains("Uniform"));
    }

    // ── DType tests ──────────────────────────────────────────────────

    #[test]
    fn test_dtype_byte_sizes() {
        assert_eq!(DType::Float32.byte_size(), 4);
        assert_eq!(DType::Float64.byte_size(), 8);
        assert_eq!(DType::Bool.byte_size(), 1);
        assert_eq!(DType::UInt8.byte_size(), 1);
        assert_eq!(DType::Int32.byte_size(), 4);
        assert_eq!(DType::Int64.byte_size(), 8);
    }

    #[test]
    fn test_dtype_labels() {
        assert_eq!(DType::Float32.label(), "float32");
        assert_eq!(DType::Float64.label(), "float64");
    }

    // ── SpaceType tests ──────────────────────────────────────────────

    #[test]
    fn test_box_space_validate() {
        let s = SpaceType::Box {
            low: vec![-1.0, -1.0, -1.0],
            high: vec![1.0, 1.0, 1.0],
            shape: vec![3],
            dtype: DType::Float32,
        };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 3);
    }

    #[test]
    fn test_box_space_invalid_bounds() {
        let s = SpaceType::Box {
            low: vec![2.0],
            high: vec![1.0],
            shape: vec![1],
            dtype: DType::Float32,
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_box_space_mismatched_shape() {
        let s = SpaceType::Box {
            low: vec![-1.0],
            high: vec![1.0],
            shape: vec![3],
            dtype: DType::Float32,
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_discrete_space() {
        let s = SpaceType::Discrete { n: 5 };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 1);
        assert!(s.contains(&[3.0]));
        assert!(!s.contains(&[5.0]));
        assert!(!s.contains(&[-1.0]));
    }

    #[test]
    fn test_discrete_zero_invalid() {
        let s = SpaceType::Discrete { n: 0 };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_multi_discrete_space() {
        let s = SpaceType::MultiDiscrete { nvec: vec![3, 4, 5] };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 3);
        assert!(s.contains(&[2.0, 3.0, 4.0]));
        assert!(!s.contains(&[3.0, 3.0, 4.0])); // 3 >= nvec[0]=3
    }

    #[test]
    fn test_multi_binary_space() {
        let s = SpaceType::MultiBinary { n: 4 };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 4);
        assert!(s.contains(&[0.0, 1.0, 1.0, 0.0]));
        assert!(!s.contains(&[0.0, 0.5, 1.0, 0.0]));
    }

    #[test]
    fn test_dict_space() {
        let mut spaces = HashMap::new();
        spaces.insert("pos".to_string(), SpaceType::Box {
            low: vec![-1.0],
            high: vec![1.0],
            shape: vec![1],
            dtype: DType::Float32,
        });
        spaces.insert("action".to_string(), SpaceType::Discrete { n: 4 });
        let s = SpaceType::Dict { spaces };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 2); // 1 + 1
    }

    #[test]
    fn test_tuple_space() {
        let s = SpaceType::Tuple {
            spaces: vec![
                SpaceType::Discrete { n: 3 },
                SpaceType::MultiBinary { n: 2 },
            ],
        };
        assert!(s.validate().is_ok());
        assert_eq!(s.flat_size(), 3); // 1 + 2
    }

    #[test]
    fn test_empty_dict_invalid() {
        let s = SpaceType::Dict { spaces: HashMap::new() };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_empty_tuple_invalid() {
        let s = SpaceType::Tuple { spaces: vec![] };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_space_labels() {
        assert_eq!(SpaceType::Discrete { n: 1 }.label(), "Discrete");
        assert_eq!(SpaceType::MultiBinary { n: 1 }.label(), "MultiBinary");
    }

    #[test]
    fn test_space_display() {
        let s = SpaceType::Discrete { n: 5 };
        assert_eq!(format!("{s}"), "Discrete(5)");
    }

    #[test]
    fn test_box_contains() {
        let s = SpaceType::Box {
            low: vec![-1.0, -2.0],
            high: vec![1.0, 2.0],
            shape: vec![2],
            dtype: DType::Float32,
        };
        assert!(s.contains(&[0.0, 0.0]));
        assert!(s.contains(&[-1.0, 2.0]));
        assert!(!s.contains(&[1.5, 0.0]));
        assert!(!s.contains(&[0.0])); // wrong length
    }

    // ── Reward tests ─────────────────────────────────────────────────

    #[test]
    fn test_reward_l2_distance() {
        let r = RewardComponent::new(
            "dist",
            1.0,
            RewardFunctionType::L2Distance { target: vec![0.0, 0.0] },
        );
        let val = r.compute(&[3.0, 4.0], &[], &[]);
        assert!((val - (-5.0)).abs() < 1e-6);
    }

    #[test]
    fn test_reward_energy_penalty() {
        let r = RewardComponent::new("energy", 1.0, RewardFunctionType::EnergyPenalty { weight: 0.5 });
        let val = r.compute(&[], &[2.0, 3.0], &[]);
        // -0.5 * (4 + 9) = -6.5
        assert!((val - (-6.5)).abs() < 1e-6);
    }

    #[test]
    fn test_reward_sparse() {
        let r = RewardComponent::new("sparse", 1.0, RewardFunctionType::Sparse { goal_threshold: 1.0 });
        assert!((r.compute(&[0.1, 0.1], &[], &[]) - 1.0).abs() < 1e-6);
        assert!((r.compute(&[5.0, 5.0], &[], &[]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_reward_sharpe_ratio() {
        let r = RewardComponent::new("sharpe", 1.0, RewardFunctionType::SharpeRatio);
        let val = r.compute(&[], &[], &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(val > 0.0); // positive returns => positive sharpe
    }

    #[test]
    fn test_reward_sharpe_empty() {
        let r = RewardComponent::new("sharpe", 1.0, RewardFunctionType::SharpeRatio);
        assert_eq!(r.compute(&[], &[], &[]), 0.0);
    }

    #[test]
    fn test_reward_clipping() {
        let r = RewardComponent::new(
            "clipped",
            1.0,
            RewardFunctionType::L2Distance { target: vec![0.0] },
        )
        .with_clip(-2.0, 0.0);
        let val = r.compute(&[100.0], &[], &[]);
        assert!((val - (-2.0)).abs() < 1e-6); // clipped to -2.0
    }

    #[test]
    fn test_reward_weight() {
        let r = RewardComponent::new(
            "dist",
            0.5,
            RewardFunctionType::L2Distance { target: vec![0.0] },
        );
        let val = r.compute(&[2.0], &[], &[]);
        assert!((val - (-1.0)).abs() < 1e-6); // 0.5 * -2.0
    }

    #[test]
    fn test_reward_dense() {
        let r = RewardComponent::new(
            "dense",
            1.0,
            RewardFunctionType::Dense { goal: vec![1.0, 1.0] },
        );
        let val = r.compute(&[1.0, 1.0], &[], &[]);
        assert!((val - 0.0).abs() < 1e-6); // at goal
    }

    // ── Safety constraints tests ─────────────────────────────────────

    #[test]
    fn test_safety_position_limit() {
        let mut c = SafetyConstraint::new(
            "pos_limit",
            SafetyConstraintType::PositionLimit { dimension: 0, min: -1.0, max: 1.0 },
            ConstraintSeverity::Hard,
        );
        assert!(c.check(&[0.0], &[]).is_none());
        assert!(c.check(&[2.0], &[]).is_some());
        assert_eq!(c.violation_count, 1);
    }

    #[test]
    fn test_safety_velocity_limit() {
        let mut c = SafetyConstraint::new(
            "vel_limit",
            SafetyConstraintType::VelocityLimit { dimension: 0, max_speed: 5.0 },
            ConstraintSeverity::Warning,
        );
        assert!(c.check(&[3.0], &[]).is_none());
        assert!(c.check(&[6.0], &[]).is_some());
    }

    #[test]
    fn test_safety_torque_limit() {
        let mut c = SafetyConstraint::new(
            "torque_limit",
            SafetyConstraintType::TorqueLimit { joint_index: 0, max_torque: 10.0 },
            ConstraintSeverity::Critical,
        );
        assert!(c.check(&[], &[5.0]).is_none());
        assert!(c.check(&[], &[15.0]).is_some());
    }

    #[test]
    fn test_safety_disabled_constraint() {
        let mut c = SafetyConstraint::new(
            "disabled",
            SafetyConstraintType::PositionLimit { dimension: 0, min: -1.0, max: 1.0 },
            ConstraintSeverity::Hard,
        );
        c.enabled = false;
        assert!(c.check(&[100.0], &[]).is_none());
    }

    #[test]
    fn test_safety_reset_violations() {
        let mut c = SafetyConstraint::new(
            "pos",
            SafetyConstraintType::PositionLimit { dimension: 0, min: -1.0, max: 1.0 },
            ConstraintSeverity::Hard,
        );
        c.check(&[2.0], &[]);
        c.check(&[3.0], &[]);
        assert_eq!(c.violation_count, 2);
        c.reset_violations();
        assert_eq!(c.violation_count, 0);
    }

    #[test]
    fn test_constraint_severity_penalty() {
        assert_eq!(ConstraintSeverity::Warning.penalty_multiplier(), 0.1);
        assert_eq!(ConstraintSeverity::Hard.penalty_multiplier(), 1.0);
        assert_eq!(ConstraintSeverity::Critical.penalty_multiplier(), 10.0);
    }

    // ── SimBackend tests ─────────────────────────────────────────────

    #[test]
    fn test_sim_backend_config_defaults() {
        let cfg = SimBackendConfig::new(SimBackendKind::MuJoCo);
        assert_eq!(cfg.timestep, 0.002);
        assert_eq!(cfg.substeps, 4);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_sim_backend_gpu_unsupported() {
        let cfg = SimBackendConfig::new(SimBackendKind::MuJoCo).with_gpu(true);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_sim_backend_gpu_supported() {
        let cfg = SimBackendConfig::new(SimBackendKind::Brax).with_gpu(true);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_sim_backend_invalid_timestep() {
        let mut cfg = SimBackendConfig::new(SimBackendKind::MuJoCo);
        cfg.timestep = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_sim_backend_labels() {
        assert_eq!(SimBackendKind::MuJoCo.label(), "MuJoCo");
        assert_eq!(SimBackendKind::PhysX.label(), "PhysX");
        assert_eq!(SimBackendKind::Brax.label(), "Brax");
        assert_eq!(SimBackendKind::Unity.label(), "Unity");
        assert_eq!(SimBackendKind::Custom.label(), "Custom");
    }

    // ── Connector tests ──────────────────────────────────────────────

    #[test]
    fn test_connector_config() {
        let c = ConnectorConfig::new("api", ConnectorProtocol::RestApi, "https://api.example.com");
        assert_eq!(c.port, 8080);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn test_connector_invalid_empty_name() {
        let c = ConnectorConfig::new("", ConnectorProtocol::Grpc, "localhost");
        assert!(c.validate().is_err());
    }

    #[test]
    fn test_connector_default_ports() {
        assert_eq!(ConnectorProtocol::RestApi.default_port(), 8080);
        assert_eq!(ConnectorProtocol::Grpc.default_port(), 50051);
        assert_eq!(ConnectorProtocol::Mqtt.default_port(), 1883);
        assert_eq!(ConnectorProtocol::WebSocket.default_port(), 8765);
        assert_eq!(ConnectorProtocol::Database.default_port(), 5432);
    }

    // ── Circuit breaker tests ────────────────────────────────────────

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, 2);
        assert!(cb.allow_request());
        assert_eq!(cb.state, CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let mut cb = CircuitBreaker::new(3, 2);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.allow_request());
        cb.record_failure();
        assert!(!cb.allow_request());
        assert_eq!(cb.state, CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Open);
        cb.try_half_open();
        assert_eq!(cb.state, CircuitBreakerState::HalfOpen);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        cb.try_half_open();
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        cb.reset();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    // ── Retry config tests ───────────────────────────────────────────

    #[test]
    fn test_retry_exponential_backoff() {
        let r = RetryConfig::new(5);
        assert_eq!(r.delay_for_retry(0), 100);
        assert_eq!(r.delay_for_retry(1), 200);
        assert_eq!(r.delay_for_retry(2), 400);
        assert_eq!(r.delay_for_retry(3), 800);
    }

    #[test]
    fn test_retry_max_delay() {
        let r = RetryConfig::new(5);
        // 100 * 2^20 >> 30000, so should be capped
        assert_eq!(r.delay_for_retry(20), 30_000);
    }

    // ── Domain randomization tests ───────────────────────────────────

    #[test]
    fn test_randomization_param() {
        let p = RandomizationParam::new("gravity", Distribution::Uniform { low: 9.0, high: 11.0 });
        assert!(p.should_apply(0));
        assert!(p.should_apply(5));
    }

    #[test]
    fn test_randomization_apply_every_n() {
        let mut p = RandomizationParam::new("mass", Distribution::Normal { mean: 1.0, std_dev: 0.1 });
        p.apply_every_n_episodes = 3;
        assert!(p.should_apply(0));
        assert!(!p.should_apply(1));
        assert!(!p.should_apply(2));
        assert!(p.should_apply(3));
    }

    #[test]
    fn test_domain_randomization_sample_all() {
        let mut dr = DomainRandomization::new(42);
        dr.add_param(RandomizationParam::new("gravity", Distribution::Uniform { low: 9.0, high: 11.0 }));
        dr.add_param(RandomizationParam::new("friction", Distribution::Uniform { low: 0.5, high: 1.5 }));
        let vals = dr.sample_all(0);
        assert_eq!(vals.len(), 2);
        assert!(vals.contains_key("gravity"));
        assert!(vals.contains_key("friction"));
    }

    #[test]
    fn test_domain_randomization_disabled() {
        let mut dr = DomainRandomization::new(42);
        dr.enabled = false;
        dr.add_param(RandomizationParam::new("x", Distribution::Uniform { low: 0.0, high: 1.0 }));
        assert!(dr.sample_all(0).is_empty());
    }

    #[test]
    fn test_domain_randomization_validate() {
        let mut dr = DomainRandomization::new(42);
        dr.add_param(RandomizationParam::new("bad", Distribution::Uniform { low: 5.0, high: 2.0 }));
        assert!(dr.validate().is_err());
    }

    // ── Trajectory/Replay tests ──────────────────────────────────────

    #[test]
    fn test_trajectory_basics() {
        let mut t = Trajectory::new("t1");
        assert!(t.is_empty());
        t.add_snapshot(StateSnapshot::new(0, vec![1.0], vec![0.5], 1.0, false));
        t.add_snapshot(StateSnapshot::new(1, vec![2.0], vec![0.3], 0.5, false));
        assert_eq!(t.len(), 2);
        assert!((t.total_reward - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_trajectory_at_step() {
        let mut t = Trajectory::new("t1");
        t.add_snapshot(StateSnapshot::new(0, vec![1.0], vec![], 1.0, false));
        t.add_snapshot(StateSnapshot::new(5, vec![2.0], vec![], 2.0, false));
        assert!(t.at_step(5).is_some());
        assert!(t.at_step(3).is_none());
    }

    #[test]
    fn test_trajectory_replay_from() {
        let mut t = Trajectory::new("t1");
        for i in 0..5 {
            t.add_snapshot(StateSnapshot::new(i, vec![i as f64], vec![], 1.0, false));
        }
        let replayed = t.replay_from(3);
        assert_eq!(replayed.len(), 2); // steps 3, 4
    }

    #[test]
    fn test_trajectory_discounted_return() {
        let mut t = Trajectory::new("t1");
        t.add_snapshot(StateSnapshot::new(0, vec![], vec![], 1.0, false));
        t.add_snapshot(StateSnapshot::new(1, vec![], vec![], 1.0, false));
        t.add_snapshot(StateSnapshot::new(2, vec![], vec![], 1.0, false));
        let ret = t.discounted_return(0, 0.9);
        // 1.0 + 0.9 + 0.81 = 2.71
        assert!((ret - 2.71).abs() < 1e-6);
    }

    #[test]
    fn test_trajectory_average_reward() {
        let mut t = Trajectory::new("t1");
        t.add_snapshot(StateSnapshot::new(0, vec![], vec![], 3.0, false));
        t.add_snapshot(StateSnapshot::new(1, vec![], vec![], 6.0, false));
        assert!((t.average_reward() - 4.5).abs() < 1e-6);
    }

    #[test]
    fn test_replay_manager() {
        let mut rm = ReplayManager::new(10);
        rm.start_trajectory("t1").unwrap();
        rm.record_step("t1", StateSnapshot::new(0, vec![1.0], vec![], 1.0, false))
            .unwrap();
        rm.save_checkpoint("cp1", "t1").unwrap();
        assert!(rm.get_trajectory("t1").is_some());
        assert!(rm.checkpoints.contains_key("cp1"));
        assert_eq!(rm.total_steps(), 1);
    }

    #[test]
    fn test_replay_manager_max_limit() {
        let mut rm = ReplayManager::new(1);
        rm.start_trajectory("t1").unwrap();
        assert!(rm.start_trajectory("t2").is_err());
    }

    #[test]
    fn test_replay_manager_missing_trajectory() {
        let mut rm = ReplayManager::new(10);
        assert!(rm.record_step("nope", StateSnapshot::new(0, vec![], vec![], 0.0, false)).is_err());
    }

    // ── Version control tests ────────────────────────────────────────

    #[test]
    fn test_version_control_commit() {
        let mut vc = EnvVersionControl::new();
        let h = vc.commit("initial", "snapshot1", 100);
        assert!(!h.is_empty());
        assert_eq!(vc.commit_count(), 1);
        assert_eq!(vc.head, Some(h));
    }

    #[test]
    fn test_version_control_tag() {
        let mut vc = EnvVersionControl::new();
        vc.commit("v1", "s1", 100);
        vc.tag("v1.0", "First release").unwrap();
        assert!(vc.tags.contains_key("v1.0"));
    }

    #[test]
    fn test_version_control_duplicate_tag() {
        let mut vc = EnvVersionControl::new();
        vc.commit("v1", "s1", 100);
        vc.tag("v1.0", "First").unwrap();
        assert!(vc.tag("v1.0", "Duplicate").is_err());
    }

    #[test]
    fn test_version_control_tag_no_commits() {
        let mut vc = EnvVersionControl::new();
        assert!(vc.tag("v1.0", "No commits").is_err());
    }

    #[test]
    fn test_version_control_rollback() {
        let mut vc = EnvVersionControl::new();
        let h1 = vc.commit("first", "s1", 100);
        let _h2 = vc.commit("second", "s2", 200);
        assert_eq!(vc.commit_count(), 2);
        vc.rollback(&h1).unwrap();
        assert_eq!(vc.commit_count(), 1);
        assert_eq!(vc.head, Some(h1));
    }

    #[test]
    fn test_version_control_diff() {
        let mut vc = EnvVersionControl::new();
        let h1 = vc.commit("v1", "line1\nline2", 100);
        let h2 = vc.commit("v2", "line1\nline2_changed", 200);
        let diffs = vc.diff(&h1, &h2).unwrap();
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].contains("line2_changed"));
    }

    #[test]
    fn test_version_control_log() {
        let mut vc = EnvVersionControl::new();
        vc.commit("first", "s1", 100);
        vc.commit("second", "s2", 200);
        let log = vc.log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].message, "second"); // newest first
    }

    // ── Env definition & DSL tests ───────────────────────────────────

    #[test]
    fn test_env_def_basics() {
        let env = EnvironmentDef::new(
            "CartPole",
            SpaceType::Box {
                low: vec![-4.8, -10.0, -0.42, -10.0],
                high: vec![4.8, 10.0, 0.42, 10.0],
                shape: vec![4],
                dtype: DType::Float32,
            },
            SpaceType::Discrete { n: 2 },
        );
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_env_def_empty_name() {
        let env = EnvironmentDef::new("", SpaceType::Discrete { n: 1 }, SpaceType::Discrete { n: 1 });
        assert!(env.validate().is_err());
    }

    #[test]
    fn test_env_def_compute_reward() {
        let mut env = EnvironmentDef::new("test", SpaceType::Discrete { n: 1 }, SpaceType::Discrete { n: 1 });
        env.add_reward(RewardComponent::new(
            "dist",
            1.0,
            RewardFunctionType::L2Distance { target: vec![0.0] },
        ));
        let r = env.compute_reward(&[3.0], &[], &[]);
        assert!((r - (-3.0)).abs() < 1e-6);
    }

    #[test]
    fn test_env_def_check_safety() {
        let mut env = EnvironmentDef::new("test", SpaceType::Discrete { n: 1 }, SpaceType::Discrete { n: 1 });
        env.add_constraint(SafetyConstraint::new(
            "pos",
            SafetyConstraintType::PositionLimit { dimension: 0, min: -1.0, max: 1.0 },
            ConstraintSeverity::Hard,
        ));
        let violations = env.check_safety(&[5.0], &[]);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_env_def_to_yaml() {
        let env = EnvironmentDef::new(
            "TestEnv",
            SpaceType::Discrete { n: 4 },
            SpaceType::Discrete { n: 2 },
        );
        let yaml = env.to_yaml();
        assert!(yaml.contains("name: TestEnv"));
        assert!(yaml.contains("observation_space:"));
    }

    #[test]
    fn test_parser_parse_space_box() {
        let s = EnvDefParser::parse_space("Box(shape=[3], low=-1.0, high=1.0, dtype=float32)").unwrap();
        if let SpaceType::Box { shape, low, high, .. } = &s {
            assert_eq!(shape, &vec![3]);
            assert_eq!(low.len(), 3);
            assert_eq!(high.len(), 3);
        } else {
            panic!("Expected Box");
        }
    }

    #[test]
    fn test_parser_parse_space_discrete() {
        let s = EnvDefParser::parse_space("Discrete(5)").unwrap();
        assert_eq!(s, SpaceType::Discrete { n: 5 });
    }

    #[test]
    fn test_parser_parse_space_multibinary() {
        let s = EnvDefParser::parse_space("MultiBinary(8)").unwrap();
        assert_eq!(s, SpaceType::MultiBinary { n: 8 });
    }

    #[test]
    fn test_parser_parse_space_multidiscrete() {
        let s = EnvDefParser::parse_space("MultiDiscrete([3, 4, 5])").unwrap();
        assert_eq!(s, SpaceType::MultiDiscrete { nvec: vec![3, 4, 5] });
    }

    #[test]
    fn test_parser_parse_full_env() {
        let input = "name: CartPole-v1\nversion: 1.0.0\nobservation_space: Box(shape=[4], low=-10.0, high=10.0)\naction_space: Discrete(2)\nmax_episode_steps: 500\ntraining_mode: Simulation\ntags: [classic, control]";
        let env = EnvDefParser::parse(input).unwrap();
        assert_eq!(env.name, "CartPole-v1");
        assert_eq!(env.version, "1.0.0");
        assert_eq!(env.max_episode_steps, 500);
        assert_eq!(env.tags, vec!["classic", "control"]);
    }

    #[test]
    fn test_parser_missing_name() {
        let input = "version: 1.0.0\nobservation_space: Discrete(4)";
        assert!(EnvDefParser::parse(input).is_err());
    }

    #[test]
    fn test_parser_unknown_space() {
        assert!(EnvDefParser::parse_space("Unknown(5)").is_err());
    }

    // ── Registry tests ───────────────────────────────────────────────

    #[test]
    fn test_registry_register_and_list() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env, "alice", 100).unwrap();
        assert_eq!(reg.count(), 1);
        assert_eq!(reg.list(), vec!["Env1"]);
    }

    #[test]
    fn test_registry_duplicate() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env.clone(), "alice", 100).unwrap();
        assert!(reg.register(env, "bob", 200).is_err());
    }

    #[test]
    fn test_registry_unregister() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env, "alice", 100).unwrap();
        reg.unregister("Env1").unwrap();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_search() {
        let mut reg = EnvironmentRegistry::new();
        let mut env = EnvironmentDef::new("CartPole", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        env.tags = vec!["classic".to_string()];
        reg.register(env, "alice", 100).unwrap();
        assert_eq!(reg.search("cart").len(), 1);
        assert_eq!(reg.search("classic").len(), 1);
        assert_eq!(reg.search("xyz").len(), 0);
    }

    #[test]
    fn test_registry_search_by_mode() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Sim1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 })
            .with_training_mode(TrainingMode::Simulation);
        reg.register(env, "alice", 100).unwrap();
        assert_eq!(reg.search_by_mode(&TrainingMode::Simulation).len(), 1);
        assert_eq!(reg.search_by_mode(&TrainingMode::RealWorld).len(), 0);
    }

    #[test]
    fn test_registry_downloads() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env, "alice", 100).unwrap();
        reg.record_download("Env1").unwrap();
        reg.record_download("Env1").unwrap();
        assert_eq!(reg.get("Env1").unwrap().downloads, 2);
    }

    #[test]
    fn test_registry_rating() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env, "alice", 100).unwrap();
        reg.set_rating("Env1", 4.5).unwrap();
        assert!((reg.get("Env1").unwrap().rating - 4.5).abs() < 1e-6);
    }

    #[test]
    fn test_registry_rating_out_of_range() {
        let mut reg = EnvironmentRegistry::new();
        let env = EnvironmentDef::new("Env1", SpaceType::Discrete { n: 2 }, SpaceType::Discrete { n: 2 });
        reg.register(env, "alice", 100).unwrap();
        assert!(reg.set_rating("Env1", 6.0).is_err());
    }

    // ── VecEnv tests ─────────────────────────────────────────────────

    #[test]
    fn test_vec_env_creation() {
        let ve = VecEnv::new(4, 3, VecEnvMode::Sync);
        assert_eq!(ve.num_envs(), 4);
        assert_eq!(ve.obs_size, 3);
    }

    #[test]
    fn test_vec_env_step_all() {
        let mut ve = VecEnv::new(2, 2, VecEnvMode::Sync);
        let actions = vec![vec![0.1, 0.1], vec![-0.1, -0.1]];
        let results = ve.step_all(&actions).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(ve.total_steps, 2);
    }

    #[test]
    fn test_vec_env_wrong_action_count() {
        let mut ve = VecEnv::new(2, 2, VecEnvMode::Sync);
        let actions = vec![vec![0.1, 0.1]]; // only 1 action for 2 envs
        assert!(ve.step_all(&actions).is_err());
    }

    #[test]
    fn test_vec_env_reset() {
        let mut ve = VecEnv::new(2, 2, VecEnvMode::Sync);
        let actions = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        ve.step_all(&actions).unwrap();
        ve.reset_all();
        let obs = ve.observations();
        assert!(obs[0].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn test_vec_env_mode_label() {
        assert_eq!(VecEnvMode::Sync.label(), "Sync");
        assert_eq!(VecEnvMode::Async.label(), "Async");
    }

    // ── Gymnasium/PettingZoo tests ───────────────────────────────────

    #[test]
    fn test_simple_gym_env() {
        let env_def = EnvironmentDef::new(
            "test",
            SpaceType::Box { low: vec![-1.0, -1.0], high: vec![1.0, 1.0], shape: vec![2], dtype: DType::Float32 },
            SpaceType::Discrete { n: 4 },
        );
        let mut gym = SimpleGymEnv::new(env_def);
        let (obs, _) = gym.reset(Some(42));
        assert_eq!(obs.len(), 2);
        let result = gym.step(&[0.1, -0.1]);
        assert_eq!(result.observation.len(), 2);
    }

    #[test]
    fn test_simple_gym_env_render() {
        let env_def = EnvironmentDef::new(
            "test",
            SpaceType::Discrete { n: 4 },
            SpaceType::Discrete { n: 2 },
        );
        let gym = SimpleGymEnv::new(env_def);
        let rendered = gym.render();
        assert!(rendered.contains("Step 0"));
    }

    #[test]
    fn test_multi_agent_env() {
        let env_def = EnvironmentDef::new(
            "multi",
            SpaceType::Box { low: vec![-1.0], high: vec![1.0], shape: vec![1], dtype: DType::Float32 },
            SpaceType::Discrete { n: 3 },
        );
        let mut ma = MultiAgentEnv::new(env_def, vec!["agent_0".into(), "agent_1".into()]);
        let obs = ma.reset_multi(None);
        assert_eq!(obs.len(), 2);
        assert_eq!(ma.agent_selection(), Some("agent_0".to_string()));
        let r = ma.step_agent("agent_0", &[0.5]);
        assert!(!r.done);
        assert_eq!(ma.agent_selection(), Some("agent_1".to_string()));
    }

    #[test]
    fn test_multi_agent_spaces() {
        let env_def = EnvironmentDef::new(
            "multi",
            SpaceType::Discrete { n: 4 },
            SpaceType::Discrete { n: 2 },
        );
        let ma = MultiAgentEnv::new(env_def, vec!["a1".into()]);
        assert!(ma.observation_space_for("a1").is_some());
        assert!(ma.action_space_for("a1").is_some());
        assert!(ma.observation_space_for("missing").is_none());
    }

    // ── Hybrid pipeline tests ────────────────────────────────────────

    #[test]
    fn test_pipeline_stages() {
        let mut p = HybridPipeline::new(0.05);
        assert_eq!(p.current_stage, PipelineStage::SimTraining);
        p.advance_stage(1.0).unwrap();
        assert_eq!(p.current_stage, PipelineStage::DomainRandomization);
    }

    #[test]
    fn test_pipeline_advance_all_stages() {
        let mut p = HybridPipeline::new(0.05);
        for _ in 0..5 {
            p.advance_stage(1.0).unwrap();
        }
        assert_eq!(p.current_stage, PipelineStage::Deployment);
        assert!(p.advance_stage(1.0).is_err());
    }

    #[test]
    fn test_adaptation_metrics() {
        let mut m = AdaptationMetrics::new(0.1);
        m.update(10.0, 9.5);
        assert!((m.transfer_gap - 0.5).abs() < 1e-6);
        assert!(!m.is_converged());
        m.update(10.0, 9.95);
        assert!(m.is_converged());
    }

    #[test]
    fn test_adaptation_efficiency() {
        let mut m = AdaptationMetrics::new(0.1);
        m.update(10.0, 8.0);
        assert!((m.transfer_efficiency() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_pipeline_sim_real_episodes() {
        let mut p = HybridPipeline::new(0.05);
        p.record_sim_episode(10.0);
        p.record_sim_episode(20.0);
        assert_eq!(p.sim_episodes, 2);
        assert!((p.metrics.sim_performance - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_pipeline_summary() {
        let p = HybridPipeline::new(0.05);
        let s = p.summary();
        assert!(s.contains("SimTraining"));
    }

    // ── Finance tests ────────────────────────────────────────────────

    #[test]
    fn test_order_book_basics() {
        let mut ob = OrderBook::new("AAPL");
        ob.add_bid(149.0, 100.0);
        ob.add_bid(148.0, 200.0);
        ob.add_ask(150.0, 50.0);
        ob.add_ask(151.0, 75.0);
        assert_eq!(ob.best_bid(), Some(149.0));
        assert_eq!(ob.best_ask(), Some(150.0));
        assert!((ob.spread().unwrap() - 1.0).abs() < 1e-6);
        assert!((ob.mid_price().unwrap() - 149.5).abs() < 1e-6);
    }

    #[test]
    fn test_order_book_fill_market_buy() {
        let mut ob = OrderBook::new("AAPL");
        ob.add_ask(150.0, 100.0);
        let mut order = Order::market(1, "AAPL", OrderSide::Buy, 50.0);
        assert!(ob.fill_market_order(&mut order));
        assert!(order.filled);
        assert_eq!(order.fill_price, Some(150.0));
        assert_eq!(ob.asks[0].quantity, 50.0);
    }

    #[test]
    fn test_order_book_fill_market_sell() {
        let mut ob = OrderBook::new("AAPL");
        ob.add_bid(149.0, 100.0);
        let mut order = Order::market(1, "AAPL", OrderSide::Sell, 30.0);
        assert!(ob.fill_market_order(&mut order));
        assert!(order.filled);
        assert_eq!(order.fill_price, Some(149.0));
    }

    #[test]
    fn test_order_book_depth() {
        let mut ob = OrderBook::new("AAPL");
        ob.add_bid(149.0, 100.0);
        ob.add_bid(148.0, 200.0);
        assert!((ob.total_bid_depth() - 300.0).abs() < 1e-6);
    }

    #[test]
    fn test_order_limit() {
        let o = Order::limit(1, "AAPL", OrderSide::Buy, 100.0, 150.0);
        assert_eq!(o.order_type, OrderType::Limit);
        assert_eq!(o.price, Some(150.0));
        assert!(!o.filled);
    }

    #[test]
    fn test_finance_reg_constraint() {
        let reg = FinanceRegConstraint::new("SEC");
        let order = Order::market(1, "AAPL", OrderSide::Buy, 500.0);
        assert!(reg.check_order(&order, 0.0).is_ok());
    }

    #[test]
    fn test_finance_reg_position_limit() {
        let reg = FinanceRegConstraint::new("SEC");
        let order = Order::market(1, "AAPL", OrderSide::Buy, 2_000_000.0);
        assert!(reg.check_order(&order, 0.0).is_err());
    }

    #[test]
    fn test_finance_reg_restricted_symbol() {
        let mut reg = FinanceRegConstraint::new("SEC");
        reg.restricted_symbols.push("RESTRICTED".to_string());
        let order = Order::market(1, "RESTRICTED", OrderSide::Buy, 1.0);
        assert!(reg.check_order(&order, 0.0).is_err());
    }

    #[test]
    fn test_finance_trading_hours() {
        let reg = FinanceRegConstraint::new("SEC");
        assert!(reg.is_within_trading_hours(600));   // 10:00 AM
        assert!(!reg.is_within_trading_hours(500));  // 8:20 AM
        assert!(!reg.is_within_trading_hours(1000)); // 4:40 PM
    }

    // ── Robotics tests ───────────────────────────────────────────────

    #[test]
    fn test_robot_joint_limits() {
        let j = RobotJoint::new("shoulder", JointType::Revolute);
        assert!(j.is_within_limits());
    }

    #[test]
    fn test_robot_joint_apply_torque() {
        let mut j = RobotJoint::new("elbow", JointType::Revolute)
            .with_limits(-1.0, 1.0, 5.0, 50.0);
        let clamped = j.apply_torque(10.0, 0.01);
        assert!(!clamped); // 10 < 50
        assert!(j.current_velocity > 0.0);
        assert!(j.current_position > 0.0);
    }

    #[test]
    fn test_robot_joint_torque_clamping() {
        let mut j = RobotJoint::new("wrist", JointType::Revolute)
            .with_limits(-1.0, 1.0, 5.0, 10.0);
        let clamped = j.apply_torque(20.0, 0.01);
        assert!(clamped); // 20 > 10
    }

    #[test]
    fn test_robot_joint_reset() {
        let mut j = RobotJoint::new("hip", JointType::Revolute);
        j.apply_torque(5.0, 0.1);
        j.reset();
        assert_eq!(j.current_position, 0.0);
        assert_eq!(j.current_velocity, 0.0);
    }

    #[test]
    fn test_robot_joint_types() {
        assert!(JointType::Revolute.has_limits());
        assert!(JointType::Prismatic.has_limits());
        assert!(!JointType::Fixed.has_limits());
        assert!(!JointType::Continuous.has_limits());
    }

    #[test]
    fn test_robotics_sim_to_real() {
        let mut r = RoboticsSimToReal::new();
        r.add_joint(RobotJoint::new("j1", JointType::Revolute));
        r.add_joint(RobotJoint::new("j2", JointType::Prismatic));
        assert!(r.all_joints_within_limits());
        assert_eq!(r.status, SimToRealStatus::SimOnly);
    }

    #[test]
    fn test_robotics_apply_torques() {
        let mut r = RoboticsSimToReal::new();
        r.add_joint(RobotJoint::new("j1", JointType::Revolute).with_limits(-1.0, 1.0, 5.0, 10.0));
        let clamped = r.apply_torques(&[20.0], 0.01);
        assert_eq!(clamped, vec![0]); // first joint was clamped
    }

    #[test]
    fn test_robotics_calibration_flow() {
        let mut r = RoboticsSimToReal::new();
        r.calibrate(0.005);
        assert_eq!(r.status, SimToRealStatus::Calibrated);
        assert!(r.validate_real());
        assert_eq!(r.status, SimToRealStatus::RealValidated);
        r.deploy().unwrap();
        assert_eq!(r.status, SimToRealStatus::Deployed);
    }

    #[test]
    fn test_robotics_deploy_without_validation() {
        let mut r = RoboticsSimToReal::new();
        assert!(r.deploy().is_err());
    }

    #[test]
    fn test_robotics_joint_positions() {
        let mut r = RoboticsSimToReal::new();
        r.add_joint(RobotJoint::new("j1", JointType::Revolute));
        r.add_joint(RobotJoint::new("j2", JointType::Revolute));
        r.apply_torques(&[1.0, -1.0], 0.1);
        let pos = r.joint_positions();
        assert_eq!(pos.len(), 2);
        assert!(pos[0] > 0.0);
        assert!(pos[1] < 0.0);
    }

    #[test]
    fn test_ros2_bridge() {
        let mut bridge = Ros2Bridge::new("rl_node", "/robot");
        bridge.add_topic("/cmd_vel", "geometry_msgs/Twist", Ros2Direction::Publish);
        bridge.add_topic("/odom", "nav_msgs/Odometry", Ros2Direction::Subscribe);
        assert_eq!(bridge.publish_topics().len(), 1);
        assert_eq!(bridge.subscribe_topics().len(), 1);
        bridge.connect().unwrap();
        assert!(bridge.connected);
        bridge.record_message();
        assert_eq!(bridge.message_count, 1);
    }

    #[test]
    fn test_ros2_bridge_empty_topics() {
        let mut bridge = Ros2Bridge::new("node", "/ns");
        assert!(bridge.connect().is_err());
    }

    #[test]
    fn test_ros2_bridge_disconnect() {
        let mut bridge = Ros2Bridge::new("node", "/ns");
        bridge.add_topic("/t", "std_msgs/String", Ros2Direction::Subscribe);
        bridge.connect().unwrap();
        bridge.disconnect();
        assert!(!bridge.connected);
    }

    // ── RlEnvOs orchestrator tests ───────────────────────────────────

    #[test]
    fn test_rl_env_os_new() {
        let os = RlEnvOs::new();
        assert_eq!(os.registry.count(), 0);
        assert_eq!(os.version_control.commit_count(), 0);
    }

    #[test]
    fn test_rl_env_os_register_env() {
        let mut os = RlEnvOs::new();
        let env = EnvironmentDef::new("TestEnv", SpaceType::Discrete { n: 4 }, SpaceType::Discrete { n: 2 });
        os.register_env(env, "alice", 100).unwrap();
        assert_eq!(os.registry.count(), 1);
        assert_eq!(os.version_control.commit_count(), 1);
    }

    #[test]
    fn test_rl_env_os_summary() {
        let os = RlEnvOs::new();
        let s = os.summary();
        assert!(s.contains("RL-OS"));
        assert!(s.contains("0 envs"));
    }

    #[test]
    fn test_rl_env_os_pipelines() {
        let mut os = RlEnvOs::new();
        os.create_pipeline("trading", 0.05);
        assert!(os.pipelines.contains_key("trading"));
    }

    #[test]
    fn test_rl_env_os_robotics() {
        let mut os = RlEnvOs::new();
        os.create_robotics("arm");
        assert!(os.robotics.contains_key("arm"));
    }

    #[test]
    fn test_rl_env_os_order_books() {
        let mut os = RlEnvOs::new();
        os.create_order_book("AAPL");
        assert!(os.order_books.contains_key("AAPL"));
    }
}
