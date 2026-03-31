#![allow(dead_code)]
//! RL-OS ServeOS — Reinforcement Learning serving operating system.
//!
//! Production-grade RL model serving with stateful session management,
//! multi-protocol routing, A/B testing, canary deployments, auto-rollback,
//! circuit breaking, edge deployment, and domain-specific integrations
//! (trading, robotics, game engines).
//!
//! # Architecture
//!
//! ```text
//! InferenceRequest (observation + session_id + metadata)
//!   → RequestRouter::route()
//!     ├─ ABTestingController::select_policy()  — traffic splitting / bandit
//!     ├─ SessionManager::get_or_create()       — per-client state
//!     ├─ InferenceEngine::infer()              — policy execution
//!     │   ├─ ObservationPreprocessor::preprocess()
//!     │   ├─ PolicyExecutor::execute()
//!     │   └─ ActionPostprocessor::postprocess()
//!     ├─ LatencySloTracker::record()           — SLO enforcement
//!     ├─ FeedbackLoop::capture()               — realized reward ingestion
//!     └─ CircuitBreaker::record_outcome()
//!   → InferenceResponse { action, metadata, latency_ms }
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ── Enums ──────────────────────────────────────────────────────────────

/// Protocol over which an inference request arrives.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ServeProtocol {
    Rest,
    Grpc,
    WebSocket,
}

impl ServeProtocol {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rest => "REST",
            Self::Grpc => "gRPC",
            Self::WebSocket => "WebSocket",
        }
    }
}

impl std::fmt::Display for ServeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Deployment target runtime environment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeploymentTarget {
    Cloud,
    EdgeWasm,
    EdgeOnnx,
    EdgeTfLite,
    EmbeddedNoStd,
}

impl DeploymentTarget {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cloud => "Cloud",
            Self::EdgeWasm => "Edge-WASM",
            Self::EdgeOnnx => "Edge-ONNX",
            Self::EdgeTfLite => "Edge-TFLite",
            Self::EmbeddedNoStd => "Embedded-NoStd",
        }
    }

    pub fn supports_gpu(&self) -> bool {
        matches!(self, Self::Cloud | Self::EdgeOnnx)
    }
}

impl std::fmt::Display for DeploymentTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Circuit breaker state machine states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Closed => "Closed",
            Self::Open => "Open",
            Self::HalfOpen => "HalfOpen",
        }
    }

    pub fn is_allowing_traffic(&self) -> bool {
        matches!(self, Self::Closed | Self::HalfOpen)
    }
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A/B testing selection strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum TrafficStrategy {
    /// Fixed percentage split across policies.
    WeightedSplit(Vec<(String, f64)>),
    /// Thompson sampling bandit.
    ThompsonSampling,
    /// Upper Confidence Bound bandit.
    Ucb,
    /// Interleaved ranking (for recommendation policies).
    Interleaving,
}

impl TrafficStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::WeightedSplit(_) => "WeightedSplit",
            Self::ThompsonSampling => "ThompsonSampling",
            Self::Ucb => "UCB",
            Self::Interleaving => "Interleaving",
        }
    }
}

impl std::fmt::Display for TrafficStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Canary rollout stage.
#[derive(Debug, Clone, PartialEq)]
pub enum CanaryStage {
    Pending,
    Stage1Pct,
    Stage5Pct,
    Stage25Pct,
    Stage100Pct,
    RolledBack,
}

impl CanaryStage {
    pub fn traffic_pct(&self) -> f64 {
        match self {
            Self::Pending => 0.0,
            Self::Stage1Pct => 1.0,
            Self::Stage5Pct => 5.0,
            Self::Stage25Pct => 25.0,
            Self::Stage100Pct => 100.0,
            Self::RolledBack => 0.0,
        }
    }

    pub fn next(&self) -> Option<CanaryStage> {
        match self {
            Self::Pending => Some(Self::Stage1Pct),
            Self::Stage1Pct => Some(Self::Stage5Pct),
            Self::Stage5Pct => Some(Self::Stage25Pct),
            Self::Stage25Pct => Some(Self::Stage100Pct),
            Self::Stage100Pct => None,
            Self::RolledBack => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Stage1Pct => "1%",
            Self::Stage5Pct => "5%",
            Self::Stage25Pct => "25%",
            Self::Stage100Pct => "100%",
            Self::RolledBack => "RolledBack",
        }
    }
}

impl std::fmt::Display for CanaryStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Deployment lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentState {
    Creating,
    Running,
    Scaling,
    Updating,
    RollingBack,
    Deleting,
    Deleted,
    Failed,
}

impl DeploymentState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Creating => "Creating",
            Self::Running => "Running",
            Self::Scaling => "Scaling",
            Self::Updating => "Updating",
            Self::RollingBack => "RollingBack",
            Self::Deleting => "Deleting",
            Self::Deleted => "Deleted",
            Self::Failed => "Failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deleted | Self::Failed)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Scaling | Self::Updating)
    }
}

impl std::fmt::Display for DeploymentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Health probe type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProbeType {
    Liveness,
    Readiness,
    Startup,
}

impl ProbeType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Liveness => "Liveness",
            Self::Readiness => "Readiness",
            Self::Startup => "Startup",
        }
    }
}

impl std::fmt::Display for ProbeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Domain-specific integration type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntegrationDomain {
    Trading,
    Robotics,
    GameUnity,
    GameGodot,
    GameCustom,
}

impl IntegrationDomain {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Trading => "Trading",
            Self::Robotics => "Robotics",
            Self::GameUnity => "Game-Unity",
            Self::GameGodot => "Game-Godot",
            Self::GameCustom => "Game-Custom",
        }
    }
}

impl std::fmt::Display for IntegrationDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Autoscaling trigger reason.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleTrigger {
    RequestRate { rps: f64, threshold: f64 },
    LatencySlo { p99_ms: f64, budget_ms: f64 },
    QueueDepth { depth: usize, threshold: usize },
    Manual { target_replicas: u32 },
}

impl ScaleTrigger {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RequestRate { .. } => "RequestRate",
            Self::LatencySlo { .. } => "LatencySLO",
            Self::QueueDepth { .. } => "QueueDepth",
            Self::Manual { .. } => "Manual",
        }
    }
}

impl std::fmt::Display for ScaleTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Core Data Structures ───────────────────────────────────────────────

/// A single observation vector with optional metadata.
#[derive(Debug, Clone)]
pub struct Observation {
    pub features: Vec<f64>,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl Observation {
    pub fn new(features: Vec<f64>) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            features,
            timestamp_ms: ts,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn dimension(&self) -> usize {
        self.features.len()
    }
}

/// An action produced by policy execution.
#[derive(Debug, Clone)]
pub struct Action {
    pub values: Vec<f64>,
    pub action_type: String,
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
}

impl Action {
    pub fn new(values: Vec<f64>, action_type: &str, confidence: f64) -> Self {
        Self {
            values,
            action_type: action_type.to_string(),
            confidence,
            metadata: HashMap::new(),
        }
    }

    pub fn discrete(index: usize, confidence: f64) -> Self {
        Self::new(vec![index as f64], "discrete", confidence)
    }

    pub fn continuous(values: Vec<f64>, confidence: f64) -> Self {
        Self::new(values, "continuous", confidence)
    }
}

/// LSTM hidden state placeholder for stateful sessions.
#[derive(Debug, Clone)]
pub struct LstmHiddenState {
    pub h: Vec<f64>,
    pub c: Vec<f64>,
    pub layer_count: usize,
}

impl LstmHiddenState {
    pub fn zeros(hidden_dim: usize, layers: usize) -> Self {
        Self {
            h: vec![0.0; hidden_dim * layers],
            c: vec![0.0; hidden_dim * layers],
            layer_count: layers,
        }
    }

    pub fn dim(&self) -> usize {
        if self.layer_count == 0 {
            0
        } else {
            self.h.len() / self.layer_count
        }
    }
}

/// Per-client session state.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub client_id: String,
    pub observation_history: Vec<Observation>,
    pub action_history: Vec<Action>,
    pub hidden_state: Option<LstmHiddenState>,
    pub episode_context: HashMap<String, String>,
    pub created_at_ms: u64,
    pub last_active_ms: u64,
    pub total_steps: u64,
    pub cumulative_reward: f64,
}

impl Session {
    pub fn new(session_id: &str, client_id: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            session_id: session_id.to_string(),
            client_id: client_id.to_string(),
            observation_history: Vec::new(),
            action_history: Vec::new(),
            hidden_state: None,
            episode_context: HashMap::new(),
            created_at_ms: now,
            last_active_ms: now,
            total_steps: 0,
            cumulative_reward: 0.0,
        }
    }

    pub fn record_step(&mut self, obs: Observation, action: Action) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.observation_history.push(obs);
        self.action_history.push(action);
        self.total_steps += 1;
        self.last_active_ms = now;
    }

    pub fn record_reward(&mut self, reward: f64) {
        self.cumulative_reward += reward;
    }

    pub fn mean_reward(&self) -> f64 {
        if self.total_steps == 0 {
            0.0
        } else {
            self.cumulative_reward / self.total_steps as f64
        }
    }

    pub fn age_ms(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now.saturating_sub(self.created_at_ms)
    }

    pub fn idle_ms(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now.saturating_sub(self.last_active_ms)
    }
}

/// Inference request from a client.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub request_id: String,
    pub session_id: String,
    pub observation: Observation,
    pub protocol: ServeProtocol,
    pub policy_hint: Option<String>,
    pub deadline_ms: Option<u64>,
}

impl InferenceRequest {
    pub fn new(request_id: &str, session_id: &str, observation: Observation) -> Self {
        Self {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            observation,
            protocol: ServeProtocol::Rest,
            deadline_ms: None,
            policy_hint: None,
        }
    }

    pub fn with_protocol(mut self, proto: ServeProtocol) -> Self {
        self.protocol = proto;
        self
    }

    pub fn with_deadline(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }
}

/// Inference response returned to the client.
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub request_id: String,
    pub action: Action,
    pub policy_id: String,
    pub policy_version: String,
    pub latency_us: u64,
    pub session_step: u64,
    pub metadata: HashMap<String, String>,
}

/// Policy configuration that the engine executes.
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    pub policy_id: String,
    pub version: String,
    pub observation_dim: usize,
    pub action_dim: usize,
    pub action_type: String,
    pub weights: Vec<f64>,
    pub bias: Vec<f64>,
    pub preprocessing: PreprocessConfig,
    pub postprocessing: PostprocessConfig,
    pub target: DeploymentTarget,
}

impl PolicyConfig {
    pub fn new(policy_id: &str, version: &str, obs_dim: usize, act_dim: usize) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            version: version.to_string(),
            observation_dim: obs_dim,
            action_dim: act_dim,
            action_type: "continuous".to_string(),
            weights: vec![0.0; obs_dim * act_dim],
            bias: vec![0.0; act_dim],
            preprocessing: PreprocessConfig::default(),
            postprocessing: PostprocessConfig::default(),
            target: DeploymentTarget::Cloud,
        }
    }

    pub fn with_weights(mut self, weights: Vec<f64>, bias: Vec<f64>) -> Self {
        self.weights = weights;
        self.bias = bias;
        self
    }

    pub fn with_target(mut self, target: DeploymentTarget) -> Self {
        self.target = target;
        self
    }
}

/// Observation preprocessing configuration.
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    pub normalize: bool,
    pub mean: Vec<f64>,
    pub std_dev: Vec<f64>,
    pub clip_min: Option<f64>,
    pub clip_max: Option<f64>,
    pub feature_select: Option<Vec<usize>>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            normalize: false,
            mean: Vec::new(),
            std_dev: Vec::new(),
            clip_min: None,
            clip_max: None,
            feature_select: None,
        }
    }
}

impl PreprocessConfig {
    pub fn with_normalization(mut self, mean: Vec<f64>, std_dev: Vec<f64>) -> Self {
        self.normalize = true;
        self.mean = mean;
        self.std_dev = std_dev;
        self
    }

    pub fn with_clipping(mut self, min: f64, max: f64) -> Self {
        self.clip_min = Some(min);
        self.clip_max = Some(max);
        self
    }
}

/// Action postprocessing configuration.
#[derive(Debug, Clone)]
pub struct PostprocessConfig {
    pub clip_min: Option<f64>,
    pub clip_max: Option<f64>,
    pub scale: f64,
    pub softmax: bool,
    pub argmax: bool,
}

impl Default for PostprocessConfig {
    fn default() -> Self {
        Self {
            clip_min: None,
            clip_max: None,
            scale: 1.0,
            softmax: false,
            argmax: false,
        }
    }
}

impl PostprocessConfig {
    pub fn with_clipping(mut self, min: f64, max: f64) -> Self {
        self.clip_min = Some(min);
        self.clip_max = Some(max);
        self
    }

    pub fn with_softmax(mut self) -> Self {
        self.softmax = true;
        self
    }

    pub fn with_argmax(mut self) -> Self {
        self.argmax = true;
        self
    }
}

/// Canary deployment descriptor.
#[derive(Debug, Clone)]
pub struct CanaryDeployment {
    pub deployment_id: String,
    pub baseline_policy: String,
    pub canary_policy: String,
    pub stage: CanaryStage,
    pub reward_threshold: f64,
    pub error_rate_threshold: f64,
    pub baseline_reward_sum: f64,
    pub canary_reward_sum: f64,
    pub baseline_count: u64,
    pub canary_count: u64,
    pub baseline_errors: u64,
    pub canary_errors: u64,
    pub created_at_ms: u64,
    pub last_promoted_ms: u64,
}

impl CanaryDeployment {
    pub fn new(
        deployment_id: &str,
        baseline: &str,
        canary: &str,
        reward_threshold: f64,
        error_rate_threshold: f64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            deployment_id: deployment_id.to_string(),
            baseline_policy: baseline.to_string(),
            canary_policy: canary.to_string(),
            stage: CanaryStage::Pending,
            reward_threshold,
            error_rate_threshold,
            baseline_reward_sum: 0.0,
            canary_reward_sum: 0.0,
            baseline_count: 0,
            canary_count: 0,
            baseline_errors: 0,
            canary_errors: 0,
            created_at_ms: now,
            last_promoted_ms: now,
        }
    }

    pub fn canary_mean_reward(&self) -> f64 {
        if self.canary_count == 0 {
            0.0
        } else {
            self.canary_reward_sum / self.canary_count as f64
        }
    }

    pub fn baseline_mean_reward(&self) -> f64 {
        if self.baseline_count == 0 {
            0.0
        } else {
            self.baseline_reward_sum / self.baseline_count as f64
        }
    }

    pub fn canary_error_rate(&self) -> f64 {
        if self.canary_count == 0 {
            0.0
        } else {
            self.canary_errors as f64 / self.canary_count as f64
        }
    }

    pub fn baseline_error_rate(&self) -> f64 {
        if self.baseline_count == 0 {
            0.0
        } else {
            self.baseline_errors as f64 / self.baseline_count as f64
        }
    }

    pub fn record_baseline(&mut self, reward: f64, is_error: bool) {
        self.baseline_reward_sum += reward;
        self.baseline_count += 1;
        if is_error {
            self.baseline_errors += 1;
        }
    }

    pub fn record_canary(&mut self, reward: f64, is_error: bool) {
        self.canary_reward_sum += reward;
        self.canary_count += 1;
        if is_error {
            self.canary_errors += 1;
        }
    }

    pub fn should_promote(&self) -> bool {
        if self.canary_count < 10 {
            return false;
        }
        let reward_ok = self.canary_mean_reward() >= self.baseline_mean_reward() * self.reward_threshold;
        let error_ok = self.canary_error_rate() <= self.error_rate_threshold;
        reward_ok && error_ok
    }

    pub fn should_rollback(&self) -> bool {
        if self.canary_count < 5 {
            return false;
        }
        self.canary_error_rate() > self.error_rate_threshold * 2.0
            || (self.canary_mean_reward() < self.baseline_mean_reward() * 0.5 && self.canary_count >= 10)
    }

    pub fn promote(&mut self) -> bool {
        if let Some(next) = self.stage.next() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.stage = next;
            self.last_promoted_ms = now;
            true
        } else {
            false
        }
    }

    pub fn rollback(&mut self) {
        self.stage = CanaryStage::RolledBack;
    }
}

/// Circuit breaker for a single policy endpoint.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub policy_id: String,
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub failure_threshold: u64,
    pub success_threshold_half_open: u64,
    pub half_open_max_requests: u64,
    pub half_open_current: u64,
    pub cooldown_ms: u64,
    pub last_failure_ms: u64,
    pub fallback_policy: Option<String>,
    pub total_requests: u64,
    pub total_failures: u64,
}

impl CircuitBreaker {
    pub fn new(policy_id: &str, failure_threshold: u64, cooldown_ms: u64) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold_half_open: 3,
            half_open_max_requests: 5,
            half_open_current: 0,
            cooldown_ms,
            last_failure_ms: 0,
            fallback_policy: None,
            total_requests: 0,
            total_failures: 0,
        }
    }

    pub fn with_fallback(mut self, fallback: &str) -> Self {
        self.fallback_policy = Some(fallback.to_string());
        self
    }

    pub fn with_half_open_config(mut self, success_threshold: u64, max_requests: u64) -> Self {
        self.success_threshold_half_open = success_threshold;
        self.half_open_max_requests = max_requests;
        self
    }

    pub fn can_execute(&self) -> bool {
        match &self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                now.saturating_sub(self.last_failure_ms) >= self.cooldown_ms
            }
            CircuitState::HalfOpen => self.half_open_current < self.half_open_max_requests,
        }
    }

    pub fn record_success(&mut self) {
        self.total_requests += 1;
        match &self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold_half_open {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.half_open_current = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.total_failures += 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_failure_ms = now;

        match &self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.half_open_current = 0;
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn try_half_open(&mut self) -> bool {
        if self.state == CircuitState::Open && self.can_execute() {
            self.state = CircuitState::HalfOpen;
            self.half_open_current = 0;
            self.success_count = 0;
            true
        } else {
            false
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_failures as f64 / self.total_requests as f64
        }
    }

    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.half_open_current = 0;
    }
}

/// Latency SLO configuration and tracker for a deployment target.
#[derive(Debug, Clone)]
pub struct LatencySloTracker {
    pub target: DeploymentTarget,
    pub budget_us: u64,
    pub latencies_us: Vec<u64>,
    pub violations: u64,
    pub total_requests: u64,
}

impl LatencySloTracker {
    pub fn new(target: DeploymentTarget, budget_us: u64) -> Self {
        Self {
            target,
            budget_us,
            latencies_us: Vec::new(),
            violations: 0,
            total_requests: 0,
        }
    }

    pub fn record(&mut self, latency_us: u64) {
        self.latencies_us.push(latency_us);
        self.total_requests += 1;
        if latency_us > self.budget_us {
            self.violations += 1;
        }
    }

    pub fn violation_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.violations as f64 / self.total_requests as f64
        }
    }

    pub fn p50_us(&self) -> u64 {
        percentile_u64(&self.latencies_us, 50.0)
    }

    pub fn p99_us(&self) -> u64 {
        percentile_u64(&self.latencies_us, 99.0)
    }

    pub fn mean_us(&self) -> f64 {
        if self.latencies_us.is_empty() {
            0.0
        } else {
            self.latencies_us.iter().sum::<u64>() as f64 / self.latencies_us.len() as f64
        }
    }

    pub fn within_slo(&self) -> bool {
        self.violation_rate() < 0.01
    }
}

/// Feedback record from production — realized reward sent back for retraining.
#[derive(Debug, Clone)]
pub struct FeedbackRecord {
    pub request_id: String,
    pub session_id: String,
    pub policy_id: String,
    pub reward: f64,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl FeedbackRecord {
    pub fn new(request_id: &str, session_id: &str, policy_id: &str, reward: f64) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            policy_id: policy_id.to_string(),
            reward,
            timestamp_ms: ts,
            metadata: HashMap::new(),
        }
    }
}

/// Batch inference request grouping multiple observations.
#[derive(Debug, Clone)]
pub struct BatchInferenceRequest {
    pub batch_id: String,
    pub requests: Vec<InferenceRequest>,
    pub max_batch_latency_us: u64,
}

impl BatchInferenceRequest {
    pub fn new(batch_id: &str, requests: Vec<InferenceRequest>, max_latency_us: u64) -> Self {
        Self {
            batch_id: batch_id.to_string(),
            requests,
            max_batch_latency_us: max_latency_us,
        }
    }

    pub fn size(&self) -> usize {
        self.requests.len()
    }
}

/// Batch inference response.
#[derive(Debug, Clone)]
pub struct BatchInferenceResponse {
    pub batch_id: String,
    pub responses: Vec<InferenceResponse>,
    pub total_latency_us: u64,
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub probe: ProbeType,
    pub healthy: bool,
    pub message: String,
    pub latency_us: u64,
    pub timestamp_ms: u64,
}

impl HealthCheckResult {
    pub fn ok(probe: ProbeType, latency_us: u64) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            probe,
            healthy: true,
            message: "OK".to_string(),
            latency_us,
            timestamp_ms: ts,
        }
    }

    pub fn fail(probe: ProbeType, message: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            probe,
            healthy: false,
            message: message.to_string(),
            latency_us: 0,
            timestamp_ms: ts,
        }
    }
}

/// Autoscaling configuration.
#[derive(Debug, Clone)]
pub struct AutoscaleConfig {
    pub deployment_id: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub target_rps_per_replica: f64,
    pub latency_slo_p99_us: u64,
    pub scale_up_cooldown_ms: u64,
    pub scale_down_cooldown_ms: u64,
    pub current_replicas: u32,
    pub last_scale_ms: u64,
}

impl AutoscaleConfig {
    pub fn new(deployment_id: &str, min: u32, max: u32, target_rps: f64) -> Self {
        Self {
            deployment_id: deployment_id.to_string(),
            min_replicas: min,
            max_replicas: max,
            target_rps_per_replica: target_rps,
            latency_slo_p99_us: 10_000,
            scale_up_cooldown_ms: 30_000,
            scale_down_cooldown_ms: 120_000,
            current_replicas: min,
            last_scale_ms: 0,
        }
    }

    pub fn desired_replicas(&self, current_rps: f64, p99_latency_us: u64) -> u32 {
        let rps_based = (current_rps / self.target_rps_per_replica).ceil() as u32;
        let latency_factor = if p99_latency_us > self.latency_slo_p99_us {
            (p99_latency_us as f64 / self.latency_slo_p99_us as f64).ceil() as u32
        } else {
            1
        };
        let desired = rps_based.max(self.current_replicas * latency_factor);
        desired.clamp(self.min_replicas, self.max_replicas)
    }

    pub fn should_scale(&self, desired: u32, now_ms: u64) -> bool {
        if desired == self.current_replicas {
            return false;
        }
        let cooldown = if desired > self.current_replicas {
            self.scale_up_cooldown_ms
        } else {
            self.scale_down_cooldown_ms
        };
        now_ms.saturating_sub(self.last_scale_ms) >= cooldown
    }

    pub fn apply_scale(&mut self, new_replicas: u32, now_ms: u64) {
        self.current_replicas = new_replicas.clamp(self.min_replicas, self.max_replicas);
        self.last_scale_ms = now_ms;
    }
}

/// Deployment lifecycle descriptor.
#[derive(Debug, Clone)]
pub struct Deployment {
    pub deployment_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub state: DeploymentState,
    pub target: DeploymentTarget,
    pub replicas: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub metadata: HashMap<String, String>,
    pub history: Vec<DeploymentEvent>,
}

impl Deployment {
    pub fn new(deployment_id: &str, policy_id: &str, version: &str, target: DeploymentTarget) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            deployment_id: deployment_id.to_string(),
            policy_id: policy_id.to_string(),
            policy_version: version.to_string(),
            state: DeploymentState::Creating,
            target,
            replicas: 1,
            created_at_ms: now,
            updated_at_ms: now,
            metadata: HashMap::new(),
            history: vec![DeploymentEvent {
                event: "created".to_string(),
                timestamp_ms: now,
                details: format!("policy={} version={}", policy_id, version),
            }],
        }
    }

    pub fn activate(&mut self) {
        self.state = DeploymentState::Running;
        self.record_event("activated", "Deployment is now running");
    }

    pub fn update_policy(&mut self, new_version: &str) {
        self.state = DeploymentState::Updating;
        self.policy_version = new_version.to_string();
        self.record_event("updating", &format!("Updating to version {}", new_version));
    }

    pub fn scale(&mut self, replicas: u32) {
        self.state = DeploymentState::Scaling;
        let old = self.replicas;
        self.replicas = replicas;
        self.record_event("scaling", &format!("{} -> {} replicas", old, replicas));
    }

    pub fn rollback(&mut self, to_version: &str) {
        self.state = DeploymentState::RollingBack;
        self.policy_version = to_version.to_string();
        self.record_event("rollback", &format!("Rolling back to {}", to_version));
    }

    pub fn delete(&mut self) {
        self.state = DeploymentState::Deleting;
        self.record_event("deleting", "Deployment marked for deletion");
    }

    pub fn mark_deleted(&mut self) {
        self.state = DeploymentState::Deleted;
        self.record_event("deleted", "Deployment deleted");
    }

    pub fn mark_failed(&mut self, reason: &str) {
        self.state = DeploymentState::Failed;
        self.record_event("failed", reason);
    }

    fn record_event(&mut self, event: &str, details: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.updated_at_ms = now;
        self.history.push(DeploymentEvent {
            event: event.to_string(),
            timestamp_ms: now,
            details: details.to_string(),
        });
    }
}

/// A recorded deployment lifecycle event.
#[derive(Debug, Clone)]
pub struct DeploymentEvent {
    pub event: String,
    pub timestamp_ms: u64,
    pub details: String,
}

/// Trading integration — FIX protocol adapter config.
#[derive(Debug, Clone)]
pub struct TradingConfig {
    pub fix_host: String,
    pub fix_port: u16,
    pub sender_comp_id: String,
    pub target_comp_id: String,
    pub heartbeat_interval_s: u32,
    pub max_order_rate: u32,
    pub tick_latency_budget_us: u64,
}

impl TradingConfig {
    pub fn new(host: &str, port: u16, sender: &str, target: &str) -> Self {
        Self {
            fix_host: host.to_string(),
            fix_port: port,
            sender_comp_id: sender.to_string(),
            target_comp_id: target.to_string(),
            heartbeat_interval_s: 30,
            max_order_rate: 100,
            tick_latency_budget_us: 500,
        }
    }
}

/// Order book entry for trading integration.
#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp_us: u64,
}

impl OrderBookEntry {
    pub fn new(symbol: &str, side: &str, price: f64, quantity: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            side: side.to_string(),
            price,
            quantity,
            timestamp_us: 0,
        }
    }

    pub fn notional(&self) -> f64 {
        self.price * self.quantity
    }
}

/// Robotics integration — ROS 2 bridge config.
#[derive(Debug, Clone)]
pub struct RoboticsConfig {
    pub ros_domain_id: u32,
    pub control_freq_hz: f64,
    pub safety_override_enabled: bool,
    pub max_velocity: f64,
    pub max_acceleration: f64,
    pub emergency_stop_topic: String,
}

impl RoboticsConfig {
    pub fn new(domain_id: u32, control_freq_hz: f64) -> Self {
        Self {
            ros_domain_id: domain_id,
            control_freq_hz,
            safety_override_enabled: true,
            max_velocity: 1.0,
            max_acceleration: 0.5,
            emergency_stop_topic: "/emergency_stop".to_string(),
        }
    }

    pub fn control_period_us(&self) -> u64 {
        if self.control_freq_hz <= 0.0 {
            return u64::MAX;
        }
        (1_000_000.0 / self.control_freq_hz) as u64
    }

    pub fn is_safe_velocity(&self, vel: f64) -> bool {
        vel.abs() <= self.max_velocity
    }

    pub fn is_safe_acceleration(&self, acc: f64) -> bool {
        acc.abs() <= self.max_acceleration
    }
}

/// Game engine integration config.
#[derive(Debug, Clone)]
pub struct GameEngineConfig {
    pub engine_type: IntegrationDomain,
    pub action_serialization: String,
    pub max_frame_budget_us: u64,
    pub observation_shape: Vec<usize>,
    pub action_space_size: usize,
}

impl GameEngineConfig {
    pub fn unity(obs_shape: Vec<usize>, action_size: usize) -> Self {
        Self {
            engine_type: IntegrationDomain::GameUnity,
            action_serialization: "json".to_string(),
            max_frame_budget_us: 16_667, // 60 FPS
            observation_shape: obs_shape,
            action_space_size: action_size,
        }
    }

    pub fn godot(obs_shape: Vec<usize>, action_size: usize) -> Self {
        Self {
            engine_type: IntegrationDomain::GameGodot,
            action_serialization: "msgpack".to_string(),
            max_frame_budget_us: 16_667,
            observation_shape: obs_shape,
            action_space_size: action_size,
        }
    }

    pub fn custom(obs_shape: Vec<usize>, action_size: usize, serialization: &str) -> Self {
        Self {
            engine_type: IntegrationDomain::GameCustom,
            action_serialization: serialization.to_string(),
            max_frame_budget_us: 33_333, // 30 FPS
            observation_shape: obs_shape,
            action_space_size: action_size,
        }
    }

    pub fn total_observation_elements(&self) -> usize {
        self.observation_shape.iter().product()
    }
}

/// A/B testing arm statistics.
#[derive(Debug, Clone)]
pub struct ArmStats {
    pub policy_id: String,
    pub pulls: u64,
    pub total_reward: f64,
    pub successes: u64,
    pub failures: u64,
}

impl ArmStats {
    pub fn new(policy_id: &str) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            pulls: 0,
            total_reward: 0.0,
            successes: 0,
            failures: 0,
        }
    }

    pub fn mean_reward(&self) -> f64 {
        if self.pulls == 0 {
            0.0
        } else {
            self.total_reward / self.pulls as f64
        }
    }

    pub fn record(&mut self, reward: f64, success: bool) {
        self.pulls += 1;
        self.total_reward += reward;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
    }

    /// UCB1 score: mean + sqrt(2 * ln(total) / pulls)
    pub fn ucb_score(&self, total_pulls: u64) -> f64 {
        if self.pulls == 0 {
            return f64::INFINITY;
        }
        let mean = self.mean_reward();
        let exploration = ((2.0 * (total_pulls as f64).ln()) / self.pulls as f64).sqrt();
        mean + exploration
    }

    /// Thompson sampling: sample from Beta(successes+1, failures+1).
    /// Approximation using the mean of the beta distribution for deterministic tests.
    pub fn thompson_score(&self) -> f64 {
        let alpha = self.successes as f64 + 1.0;
        let beta = self.failures as f64 + 1.0;
        alpha / (alpha + beta)
    }
}

/// Model hot-swap record.
#[derive(Debug, Clone)]
pub struct ModelSwapRecord {
    pub policy_id: String,
    pub old_version: String,
    pub new_version: String,
    pub timestamp_ms: u64,
    pub swap_duration_us: u64,
    pub success: bool,
    pub error: Option<String>,
}

impl ModelSwapRecord {
    pub fn success(policy_id: &str, old: &str, new: &str, duration_us: u64) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            policy_id: policy_id.to_string(),
            old_version: old.to_string(),
            new_version: new.to_string(),
            timestamp_ms: ts,
            swap_duration_us: duration_us,
            success: true,
            error: None,
        }
    }

    pub fn failure(policy_id: &str, old: &str, new: &str, error: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            policy_id: policy_id.to_string(),
            old_version: old.to_string(),
            new_version: new.to_string(),
            timestamp_ms: ts,
            swap_duration_us: 0,
            success: false,
            error: Some(error.to_string()),
        }
    }
}

/// Edge runtime abstraction.
#[derive(Debug, Clone)]
pub struct EdgeRuntime {
    pub runtime_type: DeploymentTarget,
    pub model_path: String,
    pub model_size_bytes: u64,
    pub max_batch_size: usize,
    pub quantized: bool,
}

impl EdgeRuntime {
    pub fn wasm(model_path: &str, size_bytes: u64) -> Self {
        Self {
            runtime_type: DeploymentTarget::EdgeWasm,
            model_path: model_path.to_string(),
            model_size_bytes: size_bytes,
            max_batch_size: 1,
            quantized: false,
        }
    }

    pub fn onnx(model_path: &str, size_bytes: u64) -> Self {
        Self {
            runtime_type: DeploymentTarget::EdgeOnnx,
            model_path: model_path.to_string(),
            model_size_bytes: size_bytes,
            max_batch_size: 32,
            quantized: false,
        }
    }

    pub fn tflite(model_path: &str, size_bytes: u64) -> Self {
        Self {
            runtime_type: DeploymentTarget::EdgeTfLite,
            model_path: model_path.to_string(),
            model_size_bytes: size_bytes,
            max_batch_size: 16,
            quantized: true,
        }
    }

    pub fn model_size_mb(&self) -> f64 {
        self.model_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Replay store entry for feedback loop.
#[derive(Debug, Clone)]
pub struct ReplayEntry {
    pub observation: Vec<f64>,
    pub action: Vec<f64>,
    pub reward: f64,
    pub next_observation: Option<Vec<f64>>,
    pub done: bool,
    pub policy_id: String,
    pub timestamp_ms: u64,
}

impl ReplayEntry {
    pub fn new(
        obs: Vec<f64>,
        action: Vec<f64>,
        reward: f64,
        next_obs: Option<Vec<f64>>,
        done: bool,
        policy_id: &str,
    ) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            observation: obs,
            action,
            reward,
            next_observation: next_obs,
            done,
            policy_id: policy_id.to_string(),
            timestamp_ms: ts,
        }
    }
}

// ── Helper Functions ───────────────────────────────────────────────────

fn percentile_u64(data: &[u64], pct: f64) -> u64 {
    if data.is_empty() {
        return 0;
    }
    let mut sorted = data.to_vec();
    sorted.sort_unstable();
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn softmax(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = values.iter().map(|v| (v - max_val).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        return vec![1.0 / values.len() as f64; values.len()];
    }
    exps.iter().map(|e| e / sum).collect()
}

fn argmax(values: &[f64]) -> usize {
    if values.is_empty() {
        return 0;
    }
    values
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn clip(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

// ── Observation Preprocessor ───────────────────────────────────────────

/// Preprocesses raw observations according to policy preprocessing config.
pub struct ObservationPreprocessor;

impl ObservationPreprocessor {
    pub fn preprocess(obs: &Observation, config: &PreprocessConfig) -> Vec<f64> {
        let mut features = if let Some(ref indices) = config.feature_select {
            indices
                .iter()
                .filter_map(|&i| obs.features.get(i).copied())
                .collect()
        } else {
            obs.features.clone()
        };

        if config.normalize && !config.mean.is_empty() && !config.std_dev.is_empty() {
            for (i, f) in features.iter_mut().enumerate() {
                let mean = config.mean.get(i).copied().unwrap_or(0.0);
                let std = config.std_dev.get(i).copied().unwrap_or(1.0);
                if std > 1e-8 {
                    *f = (*f - mean) / std;
                }
            }
        }

        if let (Some(min), Some(max)) = (config.clip_min, config.clip_max) {
            for f in features.iter_mut() {
                *f = clip(*f, min, max);
            }
        }

        features
    }
}

// ── Action Postprocessor ───────────────────────────────────────────────

/// Postprocesses raw action values according to policy config.
pub struct ActionPostprocessor;

impl ActionPostprocessor {
    pub fn postprocess(raw: &[f64], config: &PostprocessConfig) -> Vec<f64> {
        let mut values = raw.to_vec();

        // Scale
        if (config.scale - 1.0).abs() > 1e-8 {
            for v in values.iter_mut() {
                *v *= config.scale;
            }
        }

        // Clipping
        if let (Some(min), Some(max)) = (config.clip_min, config.clip_max) {
            for v in values.iter_mut() {
                *v = clip(*v, min, max);
            }
        }

        // Softmax
        if config.softmax {
            values = softmax(&values);
        }

        // Argmax — produce single-element vector
        if config.argmax && !values.is_empty() {
            let idx = argmax(&values);
            values = vec![idx as f64];
        }

        values
    }
}

// ── Policy Executor ────────────────────────────────────────────────────

/// Executes a linear policy (Wx + b) — placeholder for real model runtimes.
pub struct PolicyExecutor;

impl PolicyExecutor {
    /// Linear policy: action = W * obs + bias.
    /// W is stored row-major: weights[action_i * obs_dim + obs_j].
    pub fn execute(obs: &[f64], config: &PolicyConfig) -> Vec<f64> {
        let obs_dim = config.observation_dim;
        let act_dim = config.action_dim;
        let mut output = vec![0.0; act_dim];

        for i in 0..act_dim {
            let mut val = config.bias.get(i).copied().unwrap_or(0.0);
            for j in 0..obs_dim.min(obs.len()) {
                let w_idx = i * obs_dim + j;
                let w = config.weights.get(w_idx).copied().unwrap_or(0.0);
                val += w * obs[j];
            }
            output[i] = val;
        }

        output
    }
}

// ── Session Manager ────────────────────────────────────────────────────

/// Manages per-client stateful sessions.
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    max_sessions: usize,
    max_history_len: usize,
    session_ttl_ms: u64,
}

impl SessionManager {
    pub fn new(max_sessions: usize, max_history: usize, ttl_ms: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            max_sessions,
            max_history_len: max_history,
            session_ttl_ms: ttl_ms,
        }
    }

    pub fn get_or_create(&mut self, session_id: &str, client_id: &str) -> &mut Session {
        if !self.sessions.contains_key(session_id) {
            if self.sessions.len() >= self.max_sessions {
                self.evict_oldest();
            }
            let session = Session::new(session_id, client_id);
            self.sessions.insert(session_id.to_string(), session);
        }
        self.sessions.get_mut(session_id).unwrap()
    }

    pub fn get(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn remove(&mut self, session_id: &str) -> Option<Session> {
        self.sessions.remove(session_id)
    }

    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| now.saturating_sub(s.last_active_ms) > self.session_ttl_ms)
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired.len();
        for k in expired {
            self.sessions.remove(&k);
        }
        count
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .sessions
            .iter()
            .min_by_key(|(_, s)| s.last_active_ms)
            .map(|(k, _)| k.clone())
        {
            self.sessions.remove(&oldest_key);
        }
    }

    pub fn trim_histories(&mut self) {
        for session in self.sessions.values_mut() {
            if session.observation_history.len() > self.max_history_len {
                let drain_count = session.observation_history.len() - self.max_history_len;
                session.observation_history.drain(..drain_count);
            }
            if session.action_history.len() > self.max_history_len {
                let drain_count = session.action_history.len() - self.max_history_len;
                session.action_history.drain(..drain_count);
            }
        }
    }
}

// ── A/B Testing Controller ─────────────────────────────────────────────

/// Manages multi-policy traffic splitting and bandit-based selection.
pub struct ABTestingController {
    pub experiment_id: String,
    pub strategy: TrafficStrategy,
    pub arms: HashMap<String, ArmStats>,
    pub total_assignments: u64,
}

impl ABTestingController {
    pub fn new(experiment_id: &str, strategy: TrafficStrategy, policy_ids: &[&str]) -> Self {
        let mut arms = HashMap::new();
        for pid in policy_ids {
            arms.insert(pid.to_string(), ArmStats::new(pid));
        }
        Self {
            experiment_id: experiment_id.to_string(),
            strategy,
            arms,
            total_assignments: 0,
        }
    }

    /// Select a policy for the given request. Uses deterministic selection
    /// based on request_id hash for reproducibility.
    pub fn select_policy(&mut self, request_id: &str) -> Option<String> {
        if self.arms.is_empty() {
            return None;
        }

        let selected = match &self.strategy {
            TrafficStrategy::WeightedSplit(weights) => {
                self.select_weighted(request_id, weights)
            }
            TrafficStrategy::ThompsonSampling => self.select_thompson(),
            TrafficStrategy::Ucb => self.select_ucb(),
            TrafficStrategy::Interleaving => self.select_interleaving(request_id),
        };

        if let Some(ref pid) = selected {
            self.total_assignments += 1;
            if let Some(arm) = self.arms.get_mut(pid) {
                arm.pulls += 1;
            }
        }

        selected
    }

    pub fn record_reward(&mut self, policy_id: &str, reward: f64, success: bool) {
        if let Some(arm) = self.arms.get_mut(policy_id) {
            arm.total_reward += reward;
            if success {
                arm.successes += 1;
            } else {
                arm.failures += 1;
            }
        }
    }

    pub fn best_policy(&self) -> Option<String> {
        self.arms
            .values()
            .max_by(|a, b| {
                a.mean_reward()
                    .partial_cmp(&b.mean_reward())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.policy_id.clone())
    }

    fn select_weighted(&self, request_id: &str, weights: &[(String, f64)]) -> Option<String> {
        let hash = simple_hash(request_id);
        let total_weight: f64 = weights.iter().map(|(_, w)| w).sum();
        if total_weight <= 0.0 {
            return weights.first().map(|(id, _)| id.clone());
        }
        let normalized = (hash as f64) / (u64::MAX as f64) * total_weight;
        let mut cumulative = 0.0;
        for (id, w) in weights {
            cumulative += w;
            if normalized <= cumulative {
                return Some(id.clone());
            }
        }
        weights.last().map(|(id, _)| id.clone())
    }

    fn select_thompson(&self) -> Option<String> {
        self.arms
            .values()
            .max_by(|a, b| {
                a.thompson_score()
                    .partial_cmp(&b.thompson_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.policy_id.clone())
    }

    fn select_ucb(&self) -> Option<String> {
        let total = self.total_assignments.max(1);
        self.arms
            .values()
            .max_by(|a, b| {
                a.ucb_score(total)
                    .partial_cmp(&b.ucb_score(total))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.policy_id.clone())
    }

    fn select_interleaving(&self, request_id: &str) -> Option<String> {
        let hash = simple_hash(request_id);
        let policies: Vec<&String> = self.arms.keys().collect();
        if policies.is_empty() {
            return None;
        }
        let idx = (hash as usize) % policies.len();
        Some(policies[idx].clone())
    }
}

// ── Auto-Rollback Monitor ──────────────────────────────────────────────

/// Monitors policy reward regression and triggers rollback.
pub struct AutoRollbackMonitor {
    pub policy_id: String,
    pub window_size: usize,
    pub reward_history: Vec<f64>,
    pub regression_threshold: f64,
    pub last_known_good_version: String,
    pub current_version: String,
    pub rollback_triggered: bool,
    pub rollback_count: u64,
}

impl AutoRollbackMonitor {
    pub fn new(
        policy_id: &str,
        window_size: usize,
        regression_threshold: f64,
        current_version: &str,
    ) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            window_size,
            reward_history: Vec::new(),
            regression_threshold,
            last_known_good_version: current_version.to_string(),
            current_version: current_version.to_string(),
            rollback_triggered: false,
            rollback_count: 0,
        }
    }

    pub fn record_reward(&mut self, reward: f64) {
        self.reward_history.push(reward);
        if self.reward_history.len() > self.window_size * 3 {
            self.reward_history.drain(..self.window_size);
        }
    }

    pub fn check_regression(&self) -> bool {
        if self.reward_history.len() < self.window_size * 2 {
            return false;
        }
        let n = self.reward_history.len();
        let recent_start = n.saturating_sub(self.window_size);
        let baseline_start = recent_start.saturating_sub(self.window_size);

        let baseline_mean = mean_slice(&self.reward_history[baseline_start..recent_start]);
        let recent_mean = mean_slice(&self.reward_history[recent_start..]);

        if baseline_mean.abs() < 1e-8 {
            return recent_mean < -self.regression_threshold;
        }

        let relative_drop = (baseline_mean - recent_mean) / baseline_mean.abs();
        relative_drop > self.regression_threshold
    }

    pub fn trigger_rollback(&mut self) -> Option<String> {
        if self.check_regression() && !self.rollback_triggered {
            self.rollback_triggered = true;
            self.rollback_count += 1;
            Some(self.last_known_good_version.clone())
        } else {
            None
        }
    }

    pub fn confirm_good_version(&mut self, version: &str) {
        self.last_known_good_version = version.to_string();
        self.current_version = version.to_string();
        self.rollback_triggered = false;
    }
}

fn mean_slice(data: &[f64]) -> f64 {
    if data.is_empty() {
        0.0
    } else {
        data.iter().sum::<f64>() / data.len() as f64
    }
}

// ── Feedback Loop ──────────────────────────────────────────────────────

/// Captures realized rewards from production and stores replay entries.
pub struct FeedbackLoop {
    pub replay_store: Vec<ReplayEntry>,
    pub feedback_records: Vec<FeedbackRecord>,
    pub max_replay_size: usize,
    pub total_feedback: u64,
}

impl FeedbackLoop {
    pub fn new(max_replay_size: usize) -> Self {
        Self {
            replay_store: Vec::new(),
            feedback_records: Vec::new(),
            max_replay_size,
            total_feedback: 0,
        }
    }

    pub fn capture_feedback(&mut self, record: FeedbackRecord) {
        self.feedback_records.push(record);
        self.total_feedback += 1;
    }

    pub fn add_replay(&mut self, entry: ReplayEntry) {
        if self.replay_store.len() >= self.max_replay_size {
            self.replay_store.remove(0);
        }
        self.replay_store.push(entry);
    }

    pub fn replay_size(&self) -> usize {
        self.replay_store.len()
    }

    pub fn sample_batch(&self, batch_size: usize) -> Vec<&ReplayEntry> {
        // Simple uniform sampling by stride
        if self.replay_store.is_empty() || batch_size == 0 {
            return Vec::new();
        }
        let step = (self.replay_store.len() / batch_size).max(1);
        self.replay_store
            .iter()
            .step_by(step)
            .take(batch_size)
            .collect()
    }

    pub fn mean_reward(&self) -> f64 {
        if self.feedback_records.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.feedback_records.iter().map(|r| r.reward).sum();
        sum / self.feedback_records.len() as f64
    }

    pub fn reward_by_policy(&self) -> HashMap<String, f64> {
        let mut counts: HashMap<String, (f64, u64)> = HashMap::new();
        for r in &self.feedback_records {
            let entry = counts.entry(r.policy_id.clone()).or_insert((0.0, 0));
            entry.0 += r.reward;
            entry.1 += 1;
        }
        counts
            .into_iter()
            .map(|(k, (sum, count))| (k, sum / count as f64))
            .collect()
    }
}

// ── Request Router ─────────────────────────────────────────────────────

/// Routes incoming requests to the appropriate policy executor.
pub struct RequestRouter {
    pub policies: HashMap<String, PolicyConfig>,
    pub default_policy: Option<String>,
    pub protocol_stats: HashMap<ServeProtocol, u64>,
    pub total_routed: u64,
    pub total_errors: u64,
}

impl RequestRouter {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            default_policy: None,
            protocol_stats: HashMap::new(),
            total_routed: 0,
            total_errors: 0,
        }
    }

    pub fn register_policy(&mut self, config: PolicyConfig) {
        if self.default_policy.is_none() {
            self.default_policy = Some(config.policy_id.clone());
        }
        self.policies.insert(config.policy_id.clone(), config);
    }

    pub fn set_default_policy(&mut self, policy_id: &str) {
        self.default_policy = Some(policy_id.to_string());
    }

    pub fn resolve_policy(&self, request: &InferenceRequest) -> Option<&PolicyConfig> {
        if let Some(ref hint) = request.policy_hint {
            if let Some(p) = self.policies.get(hint) {
                return Some(p);
            }
        }
        if let Some(ref default) = self.default_policy {
            return self.policies.get(default);
        }
        None
    }

    pub fn route(&mut self, request: &InferenceRequest) -> Result<InferenceResponse, String> {
        self.total_routed += 1;
        *self
            .protocol_stats
            .entry(request.protocol.clone())
            .or_insert(0) += 1;

        let policy = match self.resolve_policy(request) {
            Some(p) => p.clone(),
            None => {
                self.total_errors += 1;
                return Err("No policy found for request".to_string());
            }
        };

        let start = Instant::now();

        // Preprocess
        let processed = ObservationPreprocessor::preprocess(&request.observation, &policy.preprocessing);

        // Execute
        let raw_action = PolicyExecutor::execute(&processed, &policy);

        // Postprocess
        let final_values = ActionPostprocessor::postprocess(&raw_action, &policy.postprocessing);

        let latency = start.elapsed();
        let confidence = if final_values.is_empty() {
            0.0
        } else {
            let max_val = final_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            max_val.tanh().abs()
        };

        let action = Action::new(final_values, &policy.action_type, confidence);

        Ok(InferenceResponse {
            request_id: request.request_id.clone(),
            action,
            policy_id: policy.policy_id.clone(),
            policy_version: policy.version.clone(),
            latency_us: latency.as_micros() as u64,
            session_step: 0,
            metadata: HashMap::new(),
        })
    }

    pub fn route_batch(
        &mut self,
        batch: &BatchInferenceRequest,
    ) -> BatchInferenceResponse {
        let start = Instant::now();
        let mut responses = Vec::with_capacity(batch.requests.len());

        for req in &batch.requests {
            match self.route(req) {
                Ok(resp) => responses.push(resp),
                Err(e) => {
                    // Create error response
                    responses.push(InferenceResponse {
                        request_id: req.request_id.clone(),
                        action: Action::new(Vec::new(), "error", 0.0),
                        policy_id: String::new(),
                        policy_version: String::new(),
                        latency_us: 0,
                        session_step: 0,
                        metadata: {
                            let mut m = HashMap::new();
                            m.insert("error".to_string(), e);
                            m
                        },
                    });
                }
            }
        }

        BatchInferenceResponse {
            batch_id: batch.batch_id.clone(),
            responses,
            total_latency_us: start.elapsed().as_micros() as u64,
        }
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

// ── Model Hot-Swap Manager ─────────────────────────────────────────────

/// Manages zero-downtime model updates.
pub struct ModelHotSwapManager {
    pub swap_history: Vec<ModelSwapRecord>,
    pub active_versions: HashMap<String, String>,
    pub pending_swaps: Vec<(String, String)>,
}

impl ModelHotSwapManager {
    pub fn new() -> Self {
        Self {
            swap_history: Vec::new(),
            active_versions: HashMap::new(),
            pending_swaps: Vec::new(),
        }
    }

    pub fn register_active(&mut self, policy_id: &str, version: &str) {
        self.active_versions
            .insert(policy_id.to_string(), version.to_string());
    }

    pub fn initiate_swap(&mut self, policy_id: &str, new_version: &str) {
        self.pending_swaps
            .push((policy_id.to_string(), new_version.to_string()));
    }

    pub fn execute_swap(
        &mut self,
        router: &mut RequestRouter,
        policy_id: &str,
        new_config: PolicyConfig,
    ) -> Result<ModelSwapRecord, String> {
        let start = Instant::now();
        let old_version = self
            .active_versions
            .get(policy_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let new_version = new_config.version.clone();

        // Validate new config
        if new_config.observation_dim == 0 || new_config.action_dim == 0 {
            let record =
                ModelSwapRecord::failure(policy_id, &old_version, &new_version, "Invalid dimensions");
            self.swap_history.push(record.clone());
            return Err("Invalid dimensions".to_string());
        }

        // Swap in router
        router.register_policy(new_config);
        let duration = start.elapsed().as_micros() as u64;

        self.active_versions
            .insert(policy_id.to_string(), new_version.clone());

        // Remove from pending
        self.pending_swaps
            .retain(|(pid, _)| pid != policy_id);

        let record = ModelSwapRecord::success(policy_id, &old_version, &new_version, duration);
        self.swap_history.push(record.clone());
        Ok(record)
    }

    pub fn swap_count(&self) -> usize {
        self.swap_history.len()
    }

    pub fn successful_swaps(&self) -> usize {
        self.swap_history.iter().filter(|r| r.success).count()
    }

    pub fn failed_swaps(&self) -> usize {
        self.swap_history.iter().filter(|r| !r.success).count()
    }
}

// ── Health Check Manager ───────────────────────────────────────────────

/// Manages liveness, readiness, and startup probes.
pub struct HealthCheckManager {
    pub checks: HashMap<ProbeType, Vec<HealthCheckResult>>,
    pub startup_complete: bool,
    pub ready: bool,
    pub alive: bool,
}

impl HealthCheckManager {
    pub fn new() -> Self {
        let mut checks = HashMap::new();
        checks.insert(ProbeType::Liveness, Vec::new());
        checks.insert(ProbeType::Readiness, Vec::new());
        checks.insert(ProbeType::Startup, Vec::new());
        Self {
            checks,
            startup_complete: false,
            ready: false,
            alive: true,
        }
    }

    pub fn check_liveness(&mut self) -> HealthCheckResult {
        let result = if self.alive {
            HealthCheckResult::ok(ProbeType::Liveness, 100)
        } else {
            HealthCheckResult::fail(ProbeType::Liveness, "Instance not alive")
        };
        self.checks
            .entry(ProbeType::Liveness)
            .or_default()
            .push(result.clone());
        result
    }

    pub fn check_readiness(&mut self) -> HealthCheckResult {
        let result = if self.ready && self.startup_complete {
            HealthCheckResult::ok(ProbeType::Readiness, 200)
        } else {
            HealthCheckResult::fail(ProbeType::Readiness, "Instance not ready")
        };
        self.checks
            .entry(ProbeType::Readiness)
            .or_default()
            .push(result.clone());
        result
    }

    pub fn check_startup(&mut self) -> HealthCheckResult {
        let result = if self.startup_complete {
            HealthCheckResult::ok(ProbeType::Startup, 50)
        } else {
            HealthCheckResult::fail(ProbeType::Startup, "Startup not complete")
        };
        self.checks
            .entry(ProbeType::Startup)
            .or_default()
            .push(result.clone());
        result
    }

    pub fn mark_startup_complete(&mut self) {
        self.startup_complete = true;
        self.ready = true;
    }

    pub fn mark_unhealthy(&mut self) {
        self.alive = false;
        self.ready = false;
    }

    pub fn mark_not_ready(&mut self) {
        self.ready = false;
    }

    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    pub fn total_checks(&self) -> usize {
        self.checks.values().map(|v| v.len()).sum()
    }
}

// ── Trading Engine Integration ─────────────────────────────────────────

/// Manages order book state and FIX protocol tick latency.
pub struct TradingEngine {
    pub config: TradingConfig,
    pub order_book: Vec<OrderBookEntry>,
    pub tick_latencies_us: Vec<u64>,
    pub total_orders: u64,
    pub rejected_orders: u64,
}

impl TradingEngine {
    pub fn new(config: TradingConfig) -> Self {
        Self {
            config,
            order_book: Vec::new(),
            tick_latencies_us: Vec::new(),
            total_orders: 0,
            rejected_orders: 0,
        }
    }

    pub fn submit_order(&mut self, entry: OrderBookEntry, latency_us: u64) -> bool {
        self.tick_latencies_us.push(latency_us);
        if latency_us > self.config.tick_latency_budget_us * 10 {
            self.rejected_orders += 1;
            return false;
        }
        self.total_orders += 1;
        self.order_book.push(entry);
        true
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.order_book
            .iter()
            .filter(|e| e.side == "buy")
            .map(|e| e.price)
            .fold(None, |acc, p| {
                Some(acc.map_or(p, |a: f64| a.max(p)))
            })
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.order_book
            .iter()
            .filter(|e| e.side == "sell")
            .map(|e| e.price)
            .fold(None, |acc, p| {
                Some(acc.map_or(p, |a: f64| a.min(p)))
            })
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mean_tick_latency_us(&self) -> f64 {
        if self.tick_latencies_us.is_empty() {
            0.0
        } else {
            self.tick_latencies_us.iter().sum::<u64>() as f64
                / self.tick_latencies_us.len() as f64
        }
    }

    pub fn p99_tick_latency_us(&self) -> u64 {
        percentile_u64(&self.tick_latencies_us, 99.0)
    }

    pub fn within_latency_budget(&self) -> bool {
        self.p99_tick_latency_us() <= self.config.tick_latency_budget_us
    }
}

// ── Robotics Integration ───────────────────────────────────────────────

/// ROS 2 bridge abstraction with safety overrides.
pub struct RoboticsController {
    pub config: RoboticsConfig,
    pub control_steps: u64,
    pub safety_violations: u64,
    pub emergency_stops: u64,
    pub last_command: Option<Vec<f64>>,
    pub velocity_history: Vec<f64>,
}

impl RoboticsController {
    pub fn new(config: RoboticsConfig) -> Self {
        Self {
            config,
            control_steps: 0,
            safety_violations: 0,
            emergency_stops: 0,
            last_command: None,
            velocity_history: Vec::new(),
        }
    }

    /// Send a control command, applying safety overrides.
    pub fn send_command(&mut self, command: Vec<f64>) -> Result<Vec<f64>, String> {
        self.control_steps += 1;

        if !self.config.safety_override_enabled {
            self.last_command = Some(command.clone());
            return Ok(command);
        }

        let mut safe_command = command.clone();
        let mut modified = false;

        for v in safe_command.iter_mut() {
            if !self.config.is_safe_velocity(*v) {
                *v = clip(*v, -self.config.max_velocity, self.config.max_velocity);
                modified = true;
            }
        }

        if modified {
            self.safety_violations += 1;
        }

        // Check for emergency stop condition: all velocities near max
        let all_max = safe_command
            .iter()
            .all(|v| v.abs() >= self.config.max_velocity * 0.99);
        if all_max && safe_command.len() > 1 {
            self.emergency_stops += 1;
            let zero = vec![0.0; safe_command.len()];
            self.last_command = Some(zero.clone());
            return Err("Emergency stop triggered".to_string());
        }

        for v in &safe_command {
            self.velocity_history.push(*v);
        }

        self.last_command = Some(safe_command.clone());
        Ok(safe_command)
    }

    pub fn mean_velocity(&self) -> f64 {
        if self.velocity_history.is_empty() {
            0.0
        } else {
            let sum: f64 = self.velocity_history.iter().map(|v| v.abs()).sum();
            sum / self.velocity_history.len() as f64
        }
    }

    pub fn safety_violation_rate(&self) -> f64 {
        if self.control_steps == 0 {
            0.0
        } else {
            self.safety_violations as f64 / self.control_steps as f64
        }
    }
}

// ── Game Engine Integration ────────────────────────────────────────────

/// Connects to game engines with action serialization.
pub struct GameEngineConnector {
    pub config: GameEngineConfig,
    pub frames_processed: u64,
    pub frame_latencies_us: Vec<u64>,
    pub dropped_frames: u64,
}

impl GameEngineConnector {
    pub fn new(config: GameEngineConfig) -> Self {
        Self {
            config,
            frames_processed: 0,
            frame_latencies_us: Vec::new(),
            dropped_frames: 0,
        }
    }

    pub fn process_frame(&mut self, observation: &[f64], latency_us: u64) -> Result<(), String> {
        self.frames_processed += 1;
        self.frame_latencies_us.push(latency_us);

        let expected_elements = self.config.total_observation_elements();
        if observation.len() != expected_elements {
            return Err(format!(
                "Observation dimension mismatch: expected {}, got {}",
                expected_elements,
                observation.len()
            ));
        }

        if latency_us > self.config.max_frame_budget_us {
            self.dropped_frames += 1;
        }

        Ok(())
    }

    pub fn serialize_action(&self, action: &Action) -> String {
        match self.config.action_serialization.as_str() {
            "json" => format!(
                "{{\"values\":{:?},\"type\":\"{}\",\"confidence\":{}}}",
                action.values, action.action_type, action.confidence
            ),
            "msgpack" => format!("msgpack<{} values>", action.values.len()),
            _ => format!("raw<{:?}>", action.values),
        }
    }

    pub fn frame_drop_rate(&self) -> f64 {
        if self.frames_processed == 0 {
            0.0
        } else {
            self.dropped_frames as f64 / self.frames_processed as f64
        }
    }

    pub fn mean_frame_latency_us(&self) -> f64 {
        if self.frame_latencies_us.is_empty() {
            0.0
        } else {
            self.frame_latencies_us.iter().sum::<u64>() as f64
                / self.frame_latencies_us.len() as f64
        }
    }
}

// ── Deployment Manager ─────────────────────────────────────────────────

/// Manages the full deployment lifecycle for RL serving instances.
pub struct DeploymentManager {
    pub deployments: HashMap<String, Deployment>,
    pub canaries: HashMap<String, CanaryDeployment>,
    pub autoscalers: HashMap<String, AutoscaleConfig>,
}

impl DeploymentManager {
    pub fn new() -> Self {
        Self {
            deployments: HashMap::new(),
            canaries: HashMap::new(),
            autoscalers: HashMap::new(),
        }
    }

    pub fn create_deployment(
        &mut self,
        deployment_id: &str,
        policy_id: &str,
        version: &str,
        target: DeploymentTarget,
    ) -> &Deployment {
        let deploy = Deployment::new(deployment_id, policy_id, version, target);
        self.deployments.insert(deployment_id.to_string(), deploy);
        self.deployments.get(deployment_id).unwrap()
    }

    pub fn activate(&mut self, deployment_id: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(deployment_id) {
            d.activate();
            true
        } else {
            false
        }
    }

    pub fn update_deployment(&mut self, deployment_id: &str, new_version: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(deployment_id) {
            d.update_policy(new_version);
            true
        } else {
            false
        }
    }

    pub fn scale_deployment(&mut self, deployment_id: &str, replicas: u32) -> bool {
        if let Some(d) = self.deployments.get_mut(deployment_id) {
            d.scale(replicas);
            true
        } else {
            false
        }
    }

    pub fn rollback_deployment(&mut self, deployment_id: &str, to_version: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(deployment_id) {
            d.rollback(to_version);
            true
        } else {
            false
        }
    }

    pub fn delete_deployment(&mut self, deployment_id: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(deployment_id) {
            d.delete();
            d.mark_deleted();
            true
        } else {
            false
        }
    }

    pub fn get(&self, deployment_id: &str) -> Option<&Deployment> {
        self.deployments.get(deployment_id)
    }

    pub fn active_deployments(&self) -> Vec<&Deployment> {
        self.deployments
            .values()
            .filter(|d| d.state.is_active())
            .collect()
    }

    pub fn create_canary(
        &mut self,
        deployment_id: &str,
        baseline: &str,
        canary: &str,
        reward_threshold: f64,
        error_threshold: f64,
    ) {
        let cd = CanaryDeployment::new(deployment_id, baseline, canary, reward_threshold, error_threshold);
        self.canaries.insert(deployment_id.to_string(), cd);
    }

    pub fn setup_autoscaler(
        &mut self,
        deployment_id: &str,
        min: u32,
        max: u32,
        target_rps: f64,
    ) {
        let asc = AutoscaleConfig::new(deployment_id, min, max, target_rps);
        self.autoscalers.insert(deployment_id.to_string(), asc);
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }
}

// ── ServeOS — Top-Level Engine ─────────────────────────────────────────

/// The top-level RL-OS serving engine, composing all subsystems.
pub struct RlServeOs {
    pub router: RequestRouter,
    pub sessions: SessionManager,
    pub ab_controller: Option<ABTestingController>,
    pub rollback_monitors: HashMap<String, AutoRollbackMonitor>,
    pub circuit_breakers: HashMap<String, CircuitBreaker>,
    pub slo_trackers: HashMap<DeploymentTarget, LatencySloTracker>,
    pub feedback: FeedbackLoop,
    pub health: HealthCheckManager,
    pub deployments: DeploymentManager,
    pub hot_swap: ModelHotSwapManager,
    pub trading: Option<TradingEngine>,
    pub robotics: Option<RoboticsController>,
    pub game_engine: Option<GameEngineConnector>,
    pub total_inferences: u64,
    pub total_errors: u64,
}

impl RlServeOs {
    pub fn new(max_sessions: usize, session_ttl_ms: u64) -> Self {
        Self {
            router: RequestRouter::new(),
            sessions: SessionManager::new(max_sessions, 1000, session_ttl_ms),
            ab_controller: None,
            rollback_monitors: HashMap::new(),
            circuit_breakers: HashMap::new(),
            slo_trackers: HashMap::new(),
            feedback: FeedbackLoop::new(100_000),
            health: HealthCheckManager::new(),
            deployments: DeploymentManager::new(),
            hot_swap: ModelHotSwapManager::new(),
            trading: None,
            robotics: None,
            game_engine: None,
            total_inferences: 0,
            total_errors: 0,
        }
    }

    pub fn register_policy(&mut self, config: PolicyConfig) {
        let pid = config.policy_id.clone();
        let version = config.version.clone();
        let target = config.target.clone();
        self.router.register_policy(config);
        self.hot_swap.register_active(&pid, &version);

        // Add SLO tracker for target
        let budget = match &target {
            DeploymentTarget::Cloud => 10_000,
            DeploymentTarget::EdgeOnnx => 5_000,
            DeploymentTarget::EdgeWasm => 20_000,
            DeploymentTarget::EdgeTfLite => 15_000,
            DeploymentTarget::EmbeddedNoStd => 1_000,
        };
        self.slo_trackers
            .entry(target)
            .or_insert_with_key(|t| LatencySloTracker::new(t.clone(), budget));
    }

    pub fn setup_ab_test(
        &mut self,
        experiment_id: &str,
        strategy: TrafficStrategy,
        policy_ids: &[&str],
    ) {
        self.ab_controller = Some(ABTestingController::new(experiment_id, strategy, policy_ids));
    }

    pub fn setup_circuit_breaker(&mut self, policy_id: &str, failure_threshold: u64, cooldown_ms: u64) {
        let cb = CircuitBreaker::new(policy_id, failure_threshold, cooldown_ms);
        self.circuit_breakers.insert(policy_id.to_string(), cb);
    }

    pub fn setup_rollback_monitor(
        &mut self,
        policy_id: &str,
        window_size: usize,
        threshold: f64,
        version: &str,
    ) {
        let rm = AutoRollbackMonitor::new(policy_id, window_size, threshold, version);
        self.rollback_monitors.insert(policy_id.to_string(), rm);
    }

    pub fn setup_trading(&mut self, config: TradingConfig) {
        self.trading = Some(TradingEngine::new(config));
    }

    pub fn setup_robotics(&mut self, config: RoboticsConfig) {
        self.robotics = Some(RoboticsController::new(config));
    }

    pub fn setup_game_engine(&mut self, config: GameEngineConfig) {
        self.game_engine = Some(GameEngineConnector::new(config));
    }

    /// Main inference entry point.
    pub fn infer(&mut self, request: InferenceRequest) -> Result<InferenceResponse, String> {
        self.total_inferences += 1;

        // Select policy via A/B controller if configured
        let mut req = request;
        if let Some(ref mut ab) = self.ab_controller {
            if let Some(pid) = ab.select_policy(&req.request_id) {
                req.policy_hint = Some(pid);
            }
        }

        // Check circuit breaker
        let policy_id_hint = req.policy_hint.clone().unwrap_or_default();
        if let Some(cb) = self.circuit_breakers.get(&policy_id_hint) {
            if !cb.can_execute() {
                self.total_errors += 1;
                if let Some(ref fallback) = cb.fallback_policy {
                    req.policy_hint = Some(fallback.clone());
                } else {
                    return Err(format!("Circuit open for policy {}", policy_id_hint));
                }
            }
        }

        // Route
        let result = self.router.route(&req);

        match &result {
            Ok(resp) => {
                // Record SLO
                let target = self
                    .router
                    .resolve_policy(&req)
                    .map(|p| p.target.clone())
                    .unwrap_or(DeploymentTarget::Cloud);
                if let Some(tracker) = self.slo_trackers.get_mut(&target) {
                    tracker.record(resp.latency_us);
                }

                // Record to session
                let session = self.sessions.get_or_create(&req.session_id, &req.session_id);
                session.record_step(req.observation.clone(), resp.action.clone());

                // Circuit breaker success
                if let Some(cb) = self.circuit_breakers.get_mut(&resp.policy_id) {
                    cb.record_success();
                }
            }
            Err(_) => {
                self.total_errors += 1;
                if let Some(cb) = self.circuit_breakers.get_mut(&policy_id_hint) {
                    cb.record_failure();
                }
            }
        }

        result
    }

    pub fn record_feedback(&mut self, record: FeedbackRecord) {
        let reward = record.reward;
        let policy_id = record.policy_id.clone();
        let session_id = record.session_id.clone();

        // A/B reward
        if let Some(ref mut ab) = self.ab_controller {
            ab.record_reward(&policy_id, reward, reward > 0.0);
        }

        // Rollback monitor
        if let Some(rm) = self.rollback_monitors.get_mut(&policy_id) {
            rm.record_reward(reward);
        }

        // Session reward
        if let Some(session) = self.sessions.get(&session_id) {
            // Intentionally read-only here; session reward is recorded via direct access if needed.
            let _ = session;
        }

        self.feedback.capture_feedback(record);
    }

    pub fn hot_swap_policy(&mut self, policy_id: &str, new_config: PolicyConfig) -> Result<ModelSwapRecord, String> {
        self.hot_swap.execute_swap(&mut self.router, policy_id, new_config)
    }

    pub fn check_health(&mut self) -> (HealthCheckResult, HealthCheckResult, HealthCheckResult) {
        let liveness = self.health.check_liveness();
        let readiness = self.health.check_readiness();
        let startup = self.health.check_startup();
        (liveness, readiness, startup)
    }

    pub fn startup(&mut self) {
        self.health.mark_startup_complete();
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_inferences == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_inferences as f64
        }
    }

    pub fn summary(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("total_inferences".to_string(), self.total_inferences.to_string());
        m.insert("total_errors".to_string(), self.total_errors.to_string());
        m.insert("error_rate".to_string(), format!("{:.4}", self.error_rate()));
        m.insert("active_sessions".to_string(), self.sessions.active_count().to_string());
        m.insert("registered_policies".to_string(), self.router.policy_count().to_string());
        m.insert("deployments".to_string(), self.deployments.deployment_count().to_string());
        m.insert("hot_swaps".to_string(), self.hot_swap.swap_count().to_string());
        m.insert("replay_store_size".to_string(), self.feedback.replay_size().to_string());
        m
    }
}

// ── Simple hash for deterministic routing ──────────────────────────────

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────

    fn sample_obs(dim: usize) -> Observation {
        Observation::new((0..dim).map(|i| i as f64 * 0.1).collect())
    }

    fn sample_policy(obs_dim: usize, act_dim: usize) -> PolicyConfig {
        let mut weights = vec![0.0; obs_dim * act_dim];
        // Simple identity-ish weights
        for i in 0..act_dim.min(obs_dim) {
            weights[i * obs_dim + i] = 1.0;
        }
        PolicyConfig::new("test-policy", "v1", obs_dim, act_dim)
            .with_weights(weights, vec![0.0; act_dim])
    }

    fn sample_request() -> InferenceRequest {
        InferenceRequest::new("req-1", "sess-1", sample_obs(4))
    }

    fn make_serve_os() -> RlServeOs {
        let mut os = RlServeOs::new(100, 60_000);
        os.register_policy(sample_policy(4, 2));
        os.startup();
        os
    }

    // ── Enum tests ────────────────────────────────────────────────────

    #[test]
    fn test_serve_protocol_labels() {
        assert_eq!(ServeProtocol::Rest.label(), "REST");
        assert_eq!(ServeProtocol::Grpc.label(), "gRPC");
        assert_eq!(ServeProtocol::WebSocket.label(), "WebSocket");
    }

    #[test]
    fn test_serve_protocol_display() {
        assert_eq!(format!("{}", ServeProtocol::Rest), "REST");
        assert_eq!(format!("{}", ServeProtocol::Grpc), "gRPC");
    }

    #[test]
    fn test_deployment_target_labels() {
        assert_eq!(DeploymentTarget::Cloud.label(), "Cloud");
        assert_eq!(DeploymentTarget::EdgeWasm.label(), "Edge-WASM");
        assert_eq!(DeploymentTarget::EmbeddedNoStd.label(), "Embedded-NoStd");
    }

    #[test]
    fn test_deployment_target_gpu_support() {
        assert!(DeploymentTarget::Cloud.supports_gpu());
        assert!(DeploymentTarget::EdgeOnnx.supports_gpu());
        assert!(!DeploymentTarget::EdgeWasm.supports_gpu());
        assert!(!DeploymentTarget::EmbeddedNoStd.supports_gpu());
    }

    #[test]
    fn test_circuit_state_labels() {
        assert_eq!(CircuitState::Closed.label(), "Closed");
        assert_eq!(CircuitState::Open.label(), "Open");
        assert_eq!(CircuitState::HalfOpen.label(), "HalfOpen");
    }

    #[test]
    fn test_circuit_state_allowing_traffic() {
        assert!(CircuitState::Closed.is_allowing_traffic());
        assert!(!CircuitState::Open.is_allowing_traffic());
        assert!(CircuitState::HalfOpen.is_allowing_traffic());
    }

    #[test]
    fn test_traffic_strategy_labels() {
        assert_eq!(TrafficStrategy::ThompsonSampling.label(), "ThompsonSampling");
        assert_eq!(TrafficStrategy::Ucb.label(), "UCB");
        assert_eq!(TrafficStrategy::Interleaving.label(), "Interleaving");
    }

    #[test]
    fn test_canary_stage_traffic() {
        assert_eq!(CanaryStage::Pending.traffic_pct(), 0.0);
        assert_eq!(CanaryStage::Stage1Pct.traffic_pct(), 1.0);
        assert_eq!(CanaryStage::Stage5Pct.traffic_pct(), 5.0);
        assert_eq!(CanaryStage::Stage25Pct.traffic_pct(), 25.0);
        assert_eq!(CanaryStage::Stage100Pct.traffic_pct(), 100.0);
    }

    #[test]
    fn test_canary_stage_next() {
        assert_eq!(CanaryStage::Pending.next(), Some(CanaryStage::Stage1Pct));
        assert_eq!(CanaryStage::Stage1Pct.next(), Some(CanaryStage::Stage5Pct));
        assert_eq!(CanaryStage::Stage25Pct.next(), Some(CanaryStage::Stage100Pct));
        assert_eq!(CanaryStage::Stage100Pct.next(), None);
        assert_eq!(CanaryStage::RolledBack.next(), None);
    }

    #[test]
    fn test_deployment_state_properties() {
        assert!(!DeploymentState::Running.is_terminal());
        assert!(DeploymentState::Deleted.is_terminal());
        assert!(DeploymentState::Failed.is_terminal());
        assert!(DeploymentState::Running.is_active());
        assert!(!DeploymentState::Creating.is_active());
    }

    #[test]
    fn test_probe_type_labels() {
        assert_eq!(ProbeType::Liveness.label(), "Liveness");
        assert_eq!(ProbeType::Readiness.label(), "Readiness");
        assert_eq!(ProbeType::Startup.label(), "Startup");
    }

    #[test]
    fn test_integration_domain_labels() {
        assert_eq!(IntegrationDomain::Trading.label(), "Trading");
        assert_eq!(IntegrationDomain::Robotics.label(), "Robotics");
        assert_eq!(IntegrationDomain::GameUnity.label(), "Game-Unity");
    }

    #[test]
    fn test_scale_trigger_labels() {
        let t = ScaleTrigger::RequestRate { rps: 100.0, threshold: 50.0 };
        assert_eq!(t.label(), "RequestRate");
        let t2 = ScaleTrigger::Manual { target_replicas: 3 };
        assert_eq!(t2.label(), "Manual");
    }

    // ── Observation / Action tests ────────────────────────────────────

    #[test]
    fn test_observation_new() {
        let obs = Observation::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(obs.dimension(), 3);
        assert!(obs.timestamp_ms > 0);
    }

    #[test]
    fn test_observation_with_metadata() {
        let obs = Observation::new(vec![1.0]).with_metadata("key", "value");
        assert_eq!(obs.metadata.get("key").unwrap(), "value");
    }

    #[test]
    fn test_action_discrete() {
        let a = Action::discrete(3, 0.95);
        assert_eq!(a.values, vec![3.0]);
        assert_eq!(a.action_type, "discrete");
        assert_eq!(a.confidence, 0.95);
    }

    #[test]
    fn test_action_continuous() {
        let a = Action::continuous(vec![0.1, 0.2], 0.8);
        assert_eq!(a.action_type, "continuous");
        assert_eq!(a.values.len(), 2);
    }

    // ── LSTM hidden state tests ───────────────────────────────────────

    #[test]
    fn test_lstm_hidden_zeros() {
        let h = LstmHiddenState::zeros(64, 2);
        assert_eq!(h.h.len(), 128);
        assert_eq!(h.c.len(), 128);
        assert_eq!(h.dim(), 64);
    }

    #[test]
    fn test_lstm_hidden_zero_layers() {
        let h = LstmHiddenState { h: Vec::new(), c: Vec::new(), layer_count: 0 };
        assert_eq!(h.dim(), 0);
    }

    // ── Session tests ─────────────────────────────────────────────────

    #[test]
    fn test_session_new() {
        let s = Session::new("s1", "c1");
        assert_eq!(s.session_id, "s1");
        assert_eq!(s.total_steps, 0);
        assert_eq!(s.cumulative_reward, 0.0);
    }

    #[test]
    fn test_session_record_step() {
        let mut s = Session::new("s1", "c1");
        s.record_step(sample_obs(4), Action::discrete(0, 1.0));
        assert_eq!(s.total_steps, 1);
        assert_eq!(s.observation_history.len(), 1);
        assert_eq!(s.action_history.len(), 1);
    }

    #[test]
    fn test_session_reward() {
        let mut s = Session::new("s1", "c1");
        s.record_step(sample_obs(2), Action::discrete(0, 1.0));
        s.record_reward(1.5);
        s.record_step(sample_obs(2), Action::discrete(1, 0.9));
        s.record_reward(0.5);
        assert_eq!(s.cumulative_reward, 2.0);
        assert_eq!(s.mean_reward(), 1.0);
    }

    #[test]
    fn test_session_mean_reward_empty() {
        let s = Session::new("s1", "c1");
        assert_eq!(s.mean_reward(), 0.0);
    }

    // ── Session Manager tests ─────────────────────────────────────────

    #[test]
    fn test_session_manager_create() {
        let mut mgr = SessionManager::new(10, 100, 60_000);
        let _ = mgr.get_or_create("s1", "c1");
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn test_session_manager_reuse() {
        let mut mgr = SessionManager::new(10, 100, 60_000);
        mgr.get_or_create("s1", "c1").record_reward(1.0);
        let s = mgr.get_or_create("s1", "c1");
        assert_eq!(s.cumulative_reward, 1.0);
    }

    #[test]
    fn test_session_manager_eviction() {
        let mut mgr = SessionManager::new(2, 100, 60_000);
        mgr.get_or_create("s1", "c1");
        mgr.get_or_create("s2", "c2");
        mgr.get_or_create("s3", "c3"); // should evict s1
        assert_eq!(mgr.active_count(), 2);
        assert!(mgr.get("s3").is_some());
    }

    #[test]
    fn test_session_manager_remove() {
        let mut mgr = SessionManager::new(10, 100, 60_000);
        mgr.get_or_create("s1", "c1");
        let removed = mgr.remove("s1");
        assert!(removed.is_some());
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_session_manager_trim_histories() {
        let mut mgr = SessionManager::new(10, 3, 60_000);
        let s = mgr.get_or_create("s1", "c1");
        for i in 0..10 {
            s.record_step(sample_obs(2), Action::discrete(i, 1.0));
        }
        mgr.trim_histories();
        let s = mgr.get("s1").unwrap();
        assert!(s.observation_history.len() <= 3);
        assert!(s.action_history.len() <= 3);
    }

    // ── Preprocessing tests ───────────────────────────────────────────

    #[test]
    fn test_preprocess_passthrough() {
        let obs = Observation::new(vec![1.0, 2.0, 3.0]);
        let config = PreprocessConfig::default();
        let result = ObservationPreprocessor::preprocess(&obs, &config);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_preprocess_normalization() {
        let obs = Observation::new(vec![10.0, 20.0]);
        let config = PreprocessConfig::default()
            .with_normalization(vec![5.0, 10.0], vec![5.0, 5.0]);
        let result = ObservationPreprocessor::preprocess(&obs, &config);
        assert!((result[0] - 1.0).abs() < 1e-8);
        assert!((result[1] - 2.0).abs() < 1e-8);
    }

    #[test]
    fn test_preprocess_clipping() {
        let obs = Observation::new(vec![-10.0, 5.0, 100.0]);
        let config = PreprocessConfig {
            clip_min: Some(-1.0),
            clip_max: Some(1.0),
            ..Default::default()
        };
        let result = ObservationPreprocessor::preprocess(&obs, &config);
        assert_eq!(result, vec![-1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_preprocess_feature_select() {
        let obs = Observation::new(vec![10.0, 20.0, 30.0, 40.0]);
        let config = PreprocessConfig {
            feature_select: Some(vec![0, 2]),
            ..Default::default()
        };
        let result = ObservationPreprocessor::preprocess(&obs, &config);
        assert_eq!(result, vec![10.0, 30.0]);
    }

    // ── Postprocessing tests ──────────────────────────────────────────

    #[test]
    fn test_postprocess_passthrough() {
        let config = PostprocessConfig::default();
        let result = ActionPostprocessor::postprocess(&[1.0, 2.0], &config);
        assert_eq!(result, vec![1.0, 2.0]);
    }

    #[test]
    fn test_postprocess_scale() {
        let config = PostprocessConfig { scale: 2.0, ..Default::default() };
        let result = ActionPostprocessor::postprocess(&[1.0, 3.0], &config);
        assert_eq!(result, vec![2.0, 6.0]);
    }

    #[test]
    fn test_postprocess_clipping() {
        let config = PostprocessConfig::default().with_clipping(-0.5, 0.5);
        let result = ActionPostprocessor::postprocess(&[-2.0, 0.3, 2.0], &config);
        assert_eq!(result, vec![-0.5, 0.3, 0.5]);
    }

    #[test]
    fn test_postprocess_softmax() {
        let config = PostprocessConfig::default().with_softmax();
        let result = ActionPostprocessor::postprocess(&[1.0, 2.0, 3.0], &config);
        let sum: f64 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(result[2] > result[1]);
        assert!(result[1] > result[0]);
    }

    #[test]
    fn test_postprocess_argmax() {
        let config = PostprocessConfig::default().with_argmax();
        let result = ActionPostprocessor::postprocess(&[0.1, 0.9, 0.5], &config);
        assert_eq!(result, vec![1.0]);
    }

    // ── Policy Executor tests ─────────────────────────────────────────

    #[test]
    fn test_policy_executor_identity() {
        let policy = sample_policy(3, 3);
        let obs = vec![1.0, 2.0, 3.0];
        let result = PolicyExecutor::execute(&obs, &policy);
        assert!((result[0] - 1.0).abs() < 1e-8);
        assert!((result[1] - 2.0).abs() < 1e-8);
        assert!((result[2] - 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_policy_executor_with_bias() {
        let policy = PolicyConfig::new("p1", "v1", 2, 2)
            .with_weights(vec![1.0, 0.0, 0.0, 1.0], vec![0.5, -0.5]);
        let result = PolicyExecutor::execute(&[3.0, 4.0], &policy);
        assert!((result[0] - 3.5).abs() < 1e-8);
        assert!((result[1] - 3.5).abs() < 1e-8);
    }

    #[test]
    fn test_policy_executor_empty_obs() {
        let policy = sample_policy(4, 2);
        let result = PolicyExecutor::execute(&[], &policy);
        assert_eq!(result.len(), 2);
    }

    // ── Request Router tests ──────────────────────────────────────────

    #[test]
    fn test_router_register_and_route() {
        let mut router = RequestRouter::new();
        router.register_policy(sample_policy(4, 2));
        let req = sample_request();
        let resp = router.route(&req);
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.request_id, "req-1");
        assert_eq!(resp.policy_id, "test-policy");
    }

    #[test]
    fn test_router_no_policy() {
        let mut router = RequestRouter::new();
        let req = sample_request();
        let resp = router.route(&req);
        assert!(resp.is_err());
    }

    #[test]
    fn test_router_policy_hint() {
        let mut router = RequestRouter::new();
        let p1 = PolicyConfig::new("p1", "v1", 4, 2).with_weights(vec![1.0; 8], vec![0.0; 2]);
        let p2 = PolicyConfig::new("p2", "v1", 4, 2).with_weights(vec![2.0; 8], vec![0.0; 2]);
        router.register_policy(p1);
        router.register_policy(p2);
        let mut req = sample_request();
        req.policy_hint = Some("p2".to_string());
        let resp = router.route(&req).unwrap();
        assert_eq!(resp.policy_id, "p2");
    }

    #[test]
    fn test_router_protocol_stats() {
        let mut router = RequestRouter::new();
        router.register_policy(sample_policy(4, 2));
        let req_rest = sample_request().with_protocol(ServeProtocol::Rest);
        let req_grpc = sample_request().with_protocol(ServeProtocol::Grpc);
        let _ = router.route(&req_rest);
        let _ = router.route(&req_grpc);
        assert_eq!(*router.protocol_stats.get(&ServeProtocol::Rest).unwrap_or(&0), 1);
        assert_eq!(*router.protocol_stats.get(&ServeProtocol::Grpc).unwrap_or(&0), 1);
    }

    #[test]
    fn test_router_batch() {
        let mut router = RequestRouter::new();
        router.register_policy(sample_policy(4, 2));
        let batch = BatchInferenceRequest::new(
            "batch-1",
            vec![
                InferenceRequest::new("r1", "s1", sample_obs(4)),
                InferenceRequest::new("r2", "s2", sample_obs(4)),
            ],
            100_000,
        );
        let resp = router.route_batch(&batch);
        assert_eq!(resp.batch_id, "batch-1");
        assert_eq!(resp.responses.len(), 2);
    }

    // ── Circuit Breaker tests ─────────────────────────────────────────

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new("p1", 3, 5000);
        assert_eq!(cb.state, CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let mut cb = CircuitBreaker::new("p1", 3, 5000);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_success_resets_count() {
        let mut cb = CircuitBreaker::new("p1", 3, 5000);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failure_count, 0);
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let mut cb = CircuitBreaker::new("p1", 1, 0); // 0 cooldown for immediate test
        cb.record_failure(); // opens
        assert_eq!(cb.state, CircuitState::Open);
        let ok = cb.try_half_open();
        assert!(ok);
        assert_eq!(cb.state, CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_half_open_recovers() {
        let mut cb = CircuitBreaker::new("p1", 1, 0)
            .with_half_open_config(2, 5);
        cb.record_failure(); // open
        cb.try_half_open();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_fails() {
        let mut cb = CircuitBreaker::new("p1", 1, 0);
        cb.record_failure(); // open
        cb.try_half_open();
        cb.record_failure(); // back to open
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_error_rate() {
        let mut cb = CircuitBreaker::new("p1", 10, 5000);
        for _ in 0..8 {
            cb.record_success();
        }
        for _ in 0..2 {
            cb.record_failure();
        }
        assert!((cb.error_rate() - 0.2).abs() < 1e-8);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new("p1", 1, 5000);
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        cb.reset();
        assert_eq!(cb.state, CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_circuit_breaker_with_fallback() {
        let cb = CircuitBreaker::new("p1", 3, 5000).with_fallback("p-fallback");
        assert_eq!(cb.fallback_policy, Some("p-fallback".to_string()));
    }

    // ── Canary Deployment tests ───────────────────────────────────────

    #[test]
    fn test_canary_initial_state() {
        let cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        assert_eq!(cd.stage, CanaryStage::Pending);
        assert_eq!(cd.canary_mean_reward(), 0.0);
    }

    #[test]
    fn test_canary_promotion() {
        let mut cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        assert!(cd.promote()); // Pending -> 1%
        assert_eq!(cd.stage, CanaryStage::Stage1Pct);
        assert!(cd.promote()); // 1% -> 5%
        assert_eq!(cd.stage, CanaryStage::Stage5Pct);
        assert!(cd.promote()); // 5% -> 25%
        assert!(cd.promote()); // 25% -> 100%
        assert_eq!(cd.stage, CanaryStage::Stage100Pct);
        assert!(!cd.promote()); // cannot promote further
    }

    #[test]
    fn test_canary_rollback() {
        let mut cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        cd.promote();
        cd.rollback();
        assert_eq!(cd.stage, CanaryStage::RolledBack);
    }

    #[test]
    fn test_canary_should_promote() {
        let mut cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        // Not enough data
        assert!(!cd.should_promote());
        // Add baseline
        for _ in 0..15 {
            cd.record_baseline(1.0, false);
        }
        // Add canary with good reward
        for _ in 0..15 {
            cd.record_canary(1.0, false);
        }
        assert!(cd.should_promote());
    }

    #[test]
    fn test_canary_should_rollback_high_errors() {
        let mut cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        for _ in 0..10 {
            cd.record_baseline(1.0, false);
        }
        for _ in 0..10 {
            cd.record_canary(1.0, true); // all errors
        }
        assert!(cd.should_rollback());
    }

    #[test]
    fn test_canary_error_rate() {
        let mut cd = CanaryDeployment::new("d1", "base", "canary", 0.9, 0.05);
        cd.record_canary(1.0, false);
        cd.record_canary(1.0, true);
        assert!((cd.canary_error_rate() - 0.5).abs() < 1e-8);
    }

    // ── Latency SLO Tracker tests ─────────────────────────────────────

    #[test]
    fn test_slo_tracker_basic() {
        let mut tracker = LatencySloTracker::new(DeploymentTarget::Cloud, 10_000);
        tracker.record(5_000);
        tracker.record(8_000);
        tracker.record(12_000);
        assert_eq!(tracker.violations, 1);
        assert_eq!(tracker.total_requests, 3);
    }

    #[test]
    fn test_slo_tracker_violation_rate() {
        let mut tracker = LatencySloTracker::new(DeploymentTarget::Cloud, 10_000);
        for _ in 0..90 {
            tracker.record(5_000);
        }
        for _ in 0..10 {
            tracker.record(15_000);
        }
        assert!((tracker.violation_rate() - 0.1).abs() < 1e-8);
        assert!(!tracker.within_slo());
    }

    #[test]
    fn test_slo_tracker_percentiles() {
        let mut tracker = LatencySloTracker::new(DeploymentTarget::Cloud, 10_000);
        for i in 1..=100 {
            tracker.record(i * 100);
        }
        assert!(tracker.p50_us() > 0);
        assert!(tracker.p99_us() >= tracker.p50_us());
    }

    #[test]
    fn test_slo_tracker_mean() {
        let mut tracker = LatencySloTracker::new(DeploymentTarget::Cloud, 10_000);
        tracker.record(1000);
        tracker.record(3000);
        assert!((tracker.mean_us() - 2000.0).abs() < 1e-8);
    }

    #[test]
    fn test_slo_tracker_empty() {
        let tracker = LatencySloTracker::new(DeploymentTarget::Cloud, 10_000);
        assert_eq!(tracker.violation_rate(), 0.0);
        assert_eq!(tracker.mean_us(), 0.0);
        assert!(tracker.within_slo());
    }

    // ── A/B Testing tests ─────────────────────────────────────────────

    #[test]
    fn test_ab_weighted_split() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::WeightedSplit(vec![
                ("p1".to_string(), 0.5),
                ("p2".to_string(), 0.5),
            ]),
            &["p1", "p2"],
        );
        let selected = ab.select_policy("request-abc");
        assert!(selected.is_some());
        let pid = selected.unwrap();
        assert!(pid == "p1" || pid == "p2");
    }

    #[test]
    fn test_ab_thompson() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::ThompsonSampling,
            &["p1", "p2"],
        );
        // Initially both arms equal, selection should work
        let selected = ab.select_policy("req-1");
        assert!(selected.is_some());
    }

    #[test]
    fn test_ab_ucb() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::Ucb,
            &["p1", "p2"],
        );
        let selected = ab.select_policy("req-1");
        assert!(selected.is_some());
    }

    #[test]
    fn test_ab_interleaving() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::Interleaving,
            &["p1", "p2"],
        );
        let selected = ab.select_policy("req-1");
        assert!(selected.is_some());
    }

    #[test]
    fn test_ab_record_reward() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::ThompsonSampling,
            &["p1", "p2"],
        );
        ab.record_reward("p1", 1.0, true);
        ab.record_reward("p1", 0.5, true);
        ab.record_reward("p2", 0.2, false);
        assert_eq!(ab.arms.get("p1").unwrap().total_reward, 1.5);
        assert_eq!(ab.arms.get("p2").unwrap().failures, 1);
    }

    #[test]
    fn test_ab_best_policy() {
        let mut ab = ABTestingController::new(
            "exp1",
            TrafficStrategy::Ucb,
            &["p1", "p2"],
        );
        ab.record_reward("p1", 10.0, true);
        ab.record_reward("p2", 2.0, true);
        // p1 has higher reward but no pulls, so mean_reward is tracked in total_reward
        // Since record_reward does not increment pulls, we manually adjust:
        ab.arms.get_mut("p1").unwrap().pulls = 1;
        ab.arms.get_mut("p2").unwrap().pulls = 1;
        assert_eq!(ab.best_policy(), Some("p1".to_string()));
    }

    #[test]
    fn test_ab_empty_arms() {
        let mut ab = ABTestingController::new("exp1", TrafficStrategy::Ucb, &[]);
        assert_eq!(ab.select_policy("req-1"), None);
        assert_eq!(ab.best_policy(), None);
    }

    // ── Arm Stats tests ───────────────────────────────────────────────

    #[test]
    fn test_arm_stats_mean_reward() {
        let mut arm = ArmStats::new("p1");
        arm.record(1.0, true);
        arm.record(3.0, true);
        assert!((arm.mean_reward() - 2.0).abs() < 1e-8);
    }

    #[test]
    fn test_arm_stats_ucb_infinite_for_zero_pulls() {
        let arm = ArmStats::new("p1");
        assert_eq!(arm.ucb_score(100), f64::INFINITY);
    }

    #[test]
    fn test_arm_stats_thompson() {
        let mut arm = ArmStats::new("p1");
        arm.record(1.0, true);
        arm.record(1.0, true);
        arm.record(0.0, false);
        // Beta(3, 2) mean = 3/5 = 0.6
        assert!((arm.thompson_score() - 0.6).abs() < 1e-8);
    }

    // ── Auto-Rollback Monitor tests ───────────────────────────────────

    #[test]
    fn test_rollback_no_regression() {
        let mut rm = AutoRollbackMonitor::new("p1", 5, 0.2, "v1");
        for _ in 0..10 {
            rm.record_reward(1.0);
        }
        assert!(!rm.check_regression());
    }

    #[test]
    fn test_rollback_detects_regression() {
        let mut rm = AutoRollbackMonitor::new("p1", 5, 0.2, "v1");
        // Good baseline
        for _ in 0..5 {
            rm.record_reward(1.0);
        }
        // Bad recent
        for _ in 0..5 {
            rm.record_reward(0.1);
        }
        assert!(rm.check_regression());
    }

    #[test]
    fn test_rollback_trigger() {
        let mut rm = AutoRollbackMonitor::new("p1", 5, 0.2, "v1");
        for _ in 0..5 {
            rm.record_reward(1.0);
        }
        for _ in 0..5 {
            rm.record_reward(0.1);
        }
        let result = rm.trigger_rollback();
        assert_eq!(result, Some("v1".to_string()));
        assert!(rm.rollback_triggered);
    }

    #[test]
    fn test_rollback_only_triggers_once() {
        let mut rm = AutoRollbackMonitor::new("p1", 5, 0.2, "v1");
        for _ in 0..5 {
            rm.record_reward(1.0);
        }
        for _ in 0..5 {
            rm.record_reward(0.1);
        }
        let _ = rm.trigger_rollback();
        let result = rm.trigger_rollback();
        assert_eq!(result, None); // already triggered
    }

    #[test]
    fn test_rollback_confirm_good() {
        let mut rm = AutoRollbackMonitor::new("p1", 5, 0.2, "v1");
        rm.confirm_good_version("v2");
        assert_eq!(rm.last_known_good_version, "v2");
        assert_eq!(rm.current_version, "v2");
    }

    #[test]
    fn test_rollback_not_enough_data() {
        let mut rm = AutoRollbackMonitor::new("p1", 10, 0.2, "v1");
        rm.record_reward(0.0);
        assert!(!rm.check_regression());
    }

    // ── Feedback Loop tests ───────────────────────────────────────────

    #[test]
    fn test_feedback_capture() {
        let mut fl = FeedbackLoop::new(100);
        fl.capture_feedback(FeedbackRecord::new("r1", "s1", "p1", 1.0));
        assert_eq!(fl.total_feedback, 1);
        assert_eq!(fl.feedback_records.len(), 1);
    }

    #[test]
    fn test_feedback_replay_store() {
        let mut fl = FeedbackLoop::new(3);
        for i in 0..5 {
            fl.add_replay(ReplayEntry::new(
                vec![i as f64],
                vec![0.0],
                1.0,
                None,
                false,
                "p1",
            ));
        }
        assert_eq!(fl.replay_size(), 3);
    }

    #[test]
    fn test_feedback_sample_batch() {
        let mut fl = FeedbackLoop::new(100);
        for i in 0..20 {
            fl.add_replay(ReplayEntry::new(
                vec![i as f64],
                vec![0.0],
                1.0,
                None,
                false,
                "p1",
            ));
        }
        let batch = fl.sample_batch(5);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn test_feedback_mean_reward() {
        let mut fl = FeedbackLoop::new(100);
        fl.capture_feedback(FeedbackRecord::new("r1", "s1", "p1", 2.0));
        fl.capture_feedback(FeedbackRecord::new("r2", "s1", "p1", 4.0));
        assert!((fl.mean_reward() - 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_feedback_reward_by_policy() {
        let mut fl = FeedbackLoop::new(100);
        fl.capture_feedback(FeedbackRecord::new("r1", "s1", "p1", 2.0));
        fl.capture_feedback(FeedbackRecord::new("r2", "s1", "p1", 4.0));
        fl.capture_feedback(FeedbackRecord::new("r3", "s1", "p2", 10.0));
        let by_policy = fl.reward_by_policy();
        assert!((by_policy["p1"] - 3.0).abs() < 1e-8);
        assert!((by_policy["p2"] - 10.0).abs() < 1e-8);
    }

    #[test]
    fn test_feedback_sample_empty() {
        let fl = FeedbackLoop::new(100);
        assert!(fl.sample_batch(5).is_empty());
    }

    // ── Health Check tests ────────────────────────────────────────────

    #[test]
    fn test_health_initial_state() {
        let mut hm = HealthCheckManager::new();
        assert!(!hm.startup_complete);
        let liveness = hm.check_liveness();
        assert!(liveness.healthy);
        let readiness = hm.check_readiness();
        assert!(!readiness.healthy);
    }

    #[test]
    fn test_health_startup_complete() {
        let mut hm = HealthCheckManager::new();
        hm.mark_startup_complete();
        assert!(hm.startup_complete);
        assert!(hm.ready);
        let readiness = hm.check_readiness();
        assert!(readiness.healthy);
    }

    #[test]
    fn test_health_mark_unhealthy() {
        let mut hm = HealthCheckManager::new();
        hm.mark_startup_complete();
        hm.mark_unhealthy();
        let liveness = hm.check_liveness();
        assert!(!liveness.healthy);
    }

    #[test]
    fn test_health_mark_not_ready() {
        let mut hm = HealthCheckManager::new();
        hm.mark_startup_complete();
        hm.mark_not_ready();
        let readiness = hm.check_readiness();
        assert!(!readiness.healthy);
        hm.mark_ready();
        let readiness = hm.check_readiness();
        assert!(readiness.healthy);
    }

    #[test]
    fn test_health_total_checks() {
        let mut hm = HealthCheckManager::new();
        hm.check_liveness();
        hm.check_readiness();
        hm.check_startup();
        assert_eq!(hm.total_checks(), 3);
    }

    // ── Deployment tests ──────────────────────────────────────────────

    #[test]
    fn test_deployment_lifecycle() {
        let mut d = Deployment::new("d1", "p1", "v1", DeploymentTarget::Cloud);
        assert_eq!(d.state, DeploymentState::Creating);
        d.activate();
        assert_eq!(d.state, DeploymentState::Running);
        d.scale(3);
        assert_eq!(d.state, DeploymentState::Scaling);
        assert_eq!(d.replicas, 3);
        d.update_policy("v2");
        assert_eq!(d.state, DeploymentState::Updating);
        assert_eq!(d.policy_version, "v2");
    }

    #[test]
    fn test_deployment_rollback() {
        let mut d = Deployment::new("d1", "p1", "v2", DeploymentTarget::Cloud);
        d.rollback("v1");
        assert_eq!(d.state, DeploymentState::RollingBack);
        assert_eq!(d.policy_version, "v1");
    }

    #[test]
    fn test_deployment_delete() {
        let mut d = Deployment::new("d1", "p1", "v1", DeploymentTarget::Cloud);
        d.delete();
        d.mark_deleted();
        assert_eq!(d.state, DeploymentState::Deleted);
        assert!(d.state.is_terminal());
    }

    #[test]
    fn test_deployment_history() {
        let mut d = Deployment::new("d1", "p1", "v1", DeploymentTarget::Cloud);
        d.activate();
        d.scale(5);
        assert!(d.history.len() >= 3); // created + activated + scaling
    }

    #[test]
    fn test_deployment_mark_failed() {
        let mut d = Deployment::new("d1", "p1", "v1", DeploymentTarget::Cloud);
        d.mark_failed("OOM");
        assert_eq!(d.state, DeploymentState::Failed);
    }

    // ── Deployment Manager tests ──────────────────────────────────────

    #[test]
    fn test_deployment_manager_create() {
        let mut dm = DeploymentManager::new();
        dm.create_deployment("d1", "p1", "v1", DeploymentTarget::Cloud);
        assert_eq!(dm.deployment_count(), 1);
    }

    #[test]
    fn test_deployment_manager_activate() {
        let mut dm = DeploymentManager::new();
        dm.create_deployment("d1", "p1", "v1", DeploymentTarget::Cloud);
        assert!(dm.activate("d1"));
        assert_eq!(dm.get("d1").unwrap().state, DeploymentState::Running);
    }

    #[test]
    fn test_deployment_manager_scale() {
        let mut dm = DeploymentManager::new();
        dm.create_deployment("d1", "p1", "v1", DeploymentTarget::Cloud);
        dm.activate("d1");
        dm.scale_deployment("d1", 5);
        assert_eq!(dm.get("d1").unwrap().replicas, 5);
    }

    #[test]
    fn test_deployment_manager_delete() {
        let mut dm = DeploymentManager::new();
        dm.create_deployment("d1", "p1", "v1", DeploymentTarget::Cloud);
        assert!(dm.delete_deployment("d1"));
        assert!(dm.get("d1").unwrap().state.is_terminal());
    }

    #[test]
    fn test_deployment_manager_active_deployments() {
        let mut dm = DeploymentManager::new();
        dm.create_deployment("d1", "p1", "v1", DeploymentTarget::Cloud);
        dm.create_deployment("d2", "p2", "v1", DeploymentTarget::EdgeOnnx);
        dm.activate("d1");
        assert_eq!(dm.active_deployments().len(), 1);
    }

    // ── Autoscale tests ───────────────────────────────────────────────

    #[test]
    fn test_autoscale_desired_replicas() {
        let asc = AutoscaleConfig::new("d1", 1, 10, 100.0);
        let desired = asc.desired_replicas(500.0, 5_000);
        assert_eq!(desired, 5);
    }

    #[test]
    fn test_autoscale_clamping() {
        let asc = AutoscaleConfig::new("d1", 2, 5, 100.0);
        let desired = asc.desired_replicas(1000.0, 5_000);
        assert_eq!(desired, 5); // clamped to max
        let desired2 = asc.desired_replicas(10.0, 5_000);
        assert_eq!(desired2, 2); // clamped to min
    }

    #[test]
    fn test_autoscale_should_scale() {
        let asc = AutoscaleConfig::new("d1", 1, 10, 100.0);
        assert!(asc.should_scale(5, 100_000)); // different count, past cooldown
        assert!(!asc.should_scale(1, 100_000)); // same count
    }

    #[test]
    fn test_autoscale_apply() {
        let mut asc = AutoscaleConfig::new("d1", 1, 10, 100.0);
        asc.apply_scale(5, 50_000);
        assert_eq!(asc.current_replicas, 5);
        assert_eq!(asc.last_scale_ms, 50_000);
    }

    // ── Trading Engine tests ──────────────────────────────────────────

    #[test]
    fn test_trading_submit_order() {
        let config = TradingConfig::new("localhost", 9876, "SENDER", "TARGET");
        let mut te = TradingEngine::new(config);
        let entry = OrderBookEntry::new("AAPL", "buy", 150.0, 100.0);
        assert!(te.submit_order(entry, 200));
        assert_eq!(te.total_orders, 1);
    }

    #[test]
    fn test_trading_reject_slow_order() {
        let config = TradingConfig::new("localhost", 9876, "SENDER", "TARGET");
        let mut te = TradingEngine::new(config);
        let entry = OrderBookEntry::new("AAPL", "buy", 150.0, 100.0);
        assert!(!te.submit_order(entry, 100_000)); // way over budget
        assert_eq!(te.rejected_orders, 1);
    }

    #[test]
    fn test_trading_spread() {
        let config = TradingConfig::new("localhost", 9876, "SENDER", "TARGET");
        let mut te = TradingEngine::new(config);
        te.submit_order(OrderBookEntry::new("AAPL", "buy", 149.0, 100.0), 100);
        te.submit_order(OrderBookEntry::new("AAPL", "sell", 151.0, 50.0), 100);
        assert_eq!(te.best_bid(), Some(149.0));
        assert_eq!(te.best_ask(), Some(151.0));
        assert!((te.spread().unwrap() - 2.0).abs() < 1e-8);
    }

    #[test]
    fn test_trading_notional() {
        let entry = OrderBookEntry::new("AAPL", "buy", 150.0, 100.0);
        assert!((entry.notional() - 15000.0).abs() < 1e-8);
    }

    #[test]
    fn test_trading_latency() {
        let config = TradingConfig::new("localhost", 9876, "SENDER", "TARGET");
        let mut te = TradingEngine::new(config);
        te.submit_order(OrderBookEntry::new("AAPL", "buy", 150.0, 10.0), 100);
        te.submit_order(OrderBookEntry::new("AAPL", "buy", 151.0, 10.0), 200);
        assert!((te.mean_tick_latency_us() - 150.0).abs() < 1e-8);
    }

    // ── Robotics tests ────────────────────────────────────────────────

    #[test]
    fn test_robotics_safe_command() {
        let config = RoboticsConfig::new(0, 100.0);
        let mut rc = RoboticsController::new(config);
        let result = rc.send_command(vec![0.5, -0.3]);
        assert!(result.is_ok());
        assert_eq!(rc.safety_violations, 0);
    }

    #[test]
    fn test_robotics_velocity_clipping() {
        let config = RoboticsConfig::new(0, 100.0);
        let mut rc = RoboticsController::new(config);
        let result = rc.send_command(vec![5.0, -0.3]);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert!((cmd[0] - 1.0).abs() < 1e-8); // clipped
        assert_eq!(rc.safety_violations, 1);
    }

    #[test]
    fn test_robotics_control_period() {
        let config = RoboticsConfig::new(0, 200.0);
        assert_eq!(config.control_period_us(), 5_000);
    }

    #[test]
    fn test_robotics_safety_checks() {
        let config = RoboticsConfig::new(0, 100.0);
        assert!(config.is_safe_velocity(0.5));
        assert!(!config.is_safe_velocity(1.5));
        assert!(config.is_safe_acceleration(0.3));
        assert!(!config.is_safe_acceleration(0.6));
    }

    #[test]
    fn test_robotics_safety_disabled() {
        let mut config = RoboticsConfig::new(0, 100.0);
        config.safety_override_enabled = false;
        let mut rc = RoboticsController::new(config);
        let result = rc.send_command(vec![50.0]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![50.0]); // no clipping
    }

    // ── Game Engine tests ─────────────────────────────────────────────

    #[test]
    fn test_game_engine_unity() {
        let config = GameEngineConfig::unity(vec![84, 84, 3], 5);
        assert_eq!(config.total_observation_elements(), 84 * 84 * 3);
        assert_eq!(config.engine_type, IntegrationDomain::GameUnity);
    }

    #[test]
    fn test_game_engine_godot() {
        let config = GameEngineConfig::godot(vec![10], 4);
        assert_eq!(config.action_serialization, "msgpack");
    }

    #[test]
    fn test_game_engine_process_frame() {
        let config = GameEngineConfig::custom(vec![4], 2, "json");
        let mut gc = GameEngineConnector::new(config);
        assert!(gc.process_frame(&[1.0, 2.0, 3.0, 4.0], 10_000).is_ok());
        assert_eq!(gc.frames_processed, 1);
    }

    #[test]
    fn test_game_engine_dimension_mismatch() {
        let config = GameEngineConfig::custom(vec![4], 2, "json");
        let mut gc = GameEngineConnector::new(config);
        let result = gc.process_frame(&[1.0, 2.0], 10_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_game_engine_frame_drop() {
        let config = GameEngineConfig::unity(vec![4], 2);
        let mut gc = GameEngineConnector::new(config);
        gc.process_frame(&[1.0, 2.0, 3.0, 4.0], 50_000).ok(); // over budget
        assert_eq!(gc.dropped_frames, 1);
        assert!((gc.frame_drop_rate() - 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_game_engine_serialize_json() {
        let config = GameEngineConfig::unity(vec![4], 2);
        let gc = GameEngineConnector::new(config);
        let action = Action::discrete(1, 0.9);
        let serialized = gc.serialize_action(&action);
        assert!(serialized.contains("discrete"));
    }

    // ── Model Hot-Swap tests ──────────────────────────────────────────

    #[test]
    fn test_hot_swap_register() {
        let mut hsm = ModelHotSwapManager::new();
        hsm.register_active("p1", "v1");
        assert_eq!(hsm.active_versions.get("p1").unwrap(), "v1");
    }

    #[test]
    fn test_hot_swap_execute() {
        let mut hsm = ModelHotSwapManager::new();
        let mut router = RequestRouter::new();
        router.register_policy(sample_policy(4, 2));
        hsm.register_active("test-policy", "v1");

        let new_config = PolicyConfig::new("test-policy", "v2", 4, 2)
            .with_weights(vec![2.0; 8], vec![0.0; 2]);
        let result = hsm.execute_swap(&mut router, "test-policy", new_config);
        assert!(result.is_ok());
        let record = result.unwrap();
        assert!(record.success);
        assert_eq!(record.new_version, "v2");
        assert_eq!(hsm.active_versions.get("test-policy").unwrap(), "v2");
    }

    #[test]
    fn test_hot_swap_invalid_dimensions() {
        let mut hsm = ModelHotSwapManager::new();
        let mut router = RequestRouter::new();
        let bad_config = PolicyConfig::new("p1", "v2", 0, 0);
        let result = hsm.execute_swap(&mut router, "p1", bad_config);
        assert!(result.is_err());
        assert_eq!(hsm.failed_swaps(), 1);
    }

    #[test]
    fn test_hot_swap_counts() {
        let mut hsm = ModelHotSwapManager::new();
        let mut router = RequestRouter::new();
        hsm.register_active("p1", "v1");
        let c1 = PolicyConfig::new("p1", "v2", 4, 2).with_weights(vec![0.0; 8], vec![0.0; 2]);
        let c2 = PolicyConfig::new("p1", "v3", 0, 0); // will fail
        let _ = hsm.execute_swap(&mut router, "p1", c1);
        let _ = hsm.execute_swap(&mut router, "p1", c2);
        assert_eq!(hsm.swap_count(), 2);
        assert_eq!(hsm.successful_swaps(), 1);
        assert_eq!(hsm.failed_swaps(), 1);
    }

    // ── Edge Runtime tests ────────────────────────────────────────────

    #[test]
    fn test_edge_wasm() {
        let rt = EdgeRuntime::wasm("model.wasm", 1024 * 1024);
        assert_eq!(rt.runtime_type, DeploymentTarget::EdgeWasm);
        assert!((rt.model_size_mb() - 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_edge_onnx() {
        let rt = EdgeRuntime::onnx("model.onnx", 50 * 1024 * 1024);
        assert_eq!(rt.max_batch_size, 32);
        assert!(!rt.quantized);
    }

    #[test]
    fn test_edge_tflite() {
        let rt = EdgeRuntime::tflite("model.tflite", 2 * 1024 * 1024);
        assert!(rt.quantized);
        assert_eq!(rt.max_batch_size, 16);
    }

    // ── Helper function tests ─────────────────────────────────────────

    #[test]
    fn test_softmax_values() {
        let result = softmax(&[1.0, 2.0, 3.0]);
        let sum: f64 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_softmax_empty() {
        assert!(softmax(&[]).is_empty());
    }

    #[test]
    fn test_argmax_basic() {
        assert_eq!(argmax(&[0.1, 0.9, 0.5]), 1);
        assert_eq!(argmax(&[5.0, 1.0, 2.0]), 0);
    }

    #[test]
    fn test_argmax_empty() {
        assert_eq!(argmax(&[]), 0);
    }

    #[test]
    fn test_dot_product() {
        assert!((dot_product(&[1.0, 2.0], &[3.0, 4.0]) - 11.0).abs() < 1e-8);
    }

    #[test]
    fn test_clip_function() {
        assert_eq!(clip(5.0, 0.0, 3.0), 3.0);
        assert_eq!(clip(-2.0, 0.0, 3.0), 0.0);
        assert_eq!(clip(1.5, 0.0, 3.0), 1.5);
    }

    #[test]
    fn test_percentile_u64_basic() {
        let data = vec![10, 20, 30, 40, 50];
        assert_eq!(percentile_u64(&data, 50.0), 30);
    }

    #[test]
    fn test_percentile_u64_empty() {
        assert_eq!(percentile_u64(&[], 50.0), 0);
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("hello");
        let h2 = simple_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_simple_hash_different() {
        assert_ne!(simple_hash("a"), simple_hash("b"));
    }

    // ── ServeOS integration tests ─────────────────────────────────────

    #[test]
    fn test_serve_os_basic_infer() {
        let mut os = make_serve_os();
        let req = sample_request();
        let resp = os.infer(req);
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.policy_id, "test-policy");
        assert_eq!(os.total_inferences, 1);
    }

    #[test]
    fn test_serve_os_session_created() {
        let mut os = make_serve_os();
        let _ = os.infer(sample_request());
        assert_eq!(os.sessions.active_count(), 1);
    }

    #[test]
    fn test_serve_os_error_rate() {
        let os = make_serve_os();
        assert_eq!(os.error_rate(), 0.0);
    }

    #[test]
    fn test_serve_os_with_ab_test() {
        let mut os = make_serve_os();
        let p2 = PolicyConfig::new("p2", "v1", 4, 2)
            .with_weights(vec![0.5; 8], vec![0.0; 2]);
        os.register_policy(p2);
        os.setup_ab_test(
            "exp1",
            TrafficStrategy::WeightedSplit(vec![
                ("test-policy".to_string(), 0.5),
                ("p2".to_string(), 0.5),
            ]),
            &["test-policy", "p2"],
        );
        let resp = os.infer(sample_request());
        assert!(resp.is_ok());
    }

    #[test]
    fn test_serve_os_health() {
        let mut os = make_serve_os();
        let (live, ready, startup) = os.check_health();
        assert!(live.healthy);
        assert!(ready.healthy);
        assert!(startup.healthy);
    }

    #[test]
    fn test_serve_os_feedback() {
        let mut os = make_serve_os();
        let _ = os.infer(sample_request());
        os.record_feedback(FeedbackRecord::new("req-1", "sess-1", "test-policy", 1.0));
        assert_eq!(os.feedback.total_feedback, 1);
    }

    #[test]
    fn test_serve_os_hot_swap() {
        let mut os = make_serve_os();
        let new_config = PolicyConfig::new("test-policy", "v2", 4, 2)
            .with_weights(vec![3.0; 8], vec![1.0; 2]);
        let result = os.hot_swap_policy("test-policy", new_config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serve_os_summary() {
        let mut os = make_serve_os();
        let _ = os.infer(sample_request());
        let summary = os.summary();
        assert_eq!(summary["total_inferences"], "1");
        assert!(summary.contains_key("error_rate"));
        assert!(summary.contains_key("active_sessions"));
    }

    #[test]
    fn test_serve_os_circuit_breaker_integration() {
        let mut os = make_serve_os();
        os.setup_circuit_breaker("test-policy", 3, 5000);
        let resp = os.infer(sample_request());
        assert!(resp.is_ok());
    }

    #[test]
    fn test_serve_os_rollback_monitor_integration() {
        let mut os = make_serve_os();
        os.setup_rollback_monitor("test-policy", 5, 0.2, "v1");
        os.record_feedback(FeedbackRecord::new("r1", "s1", "test-policy", 1.0));
        assert!(!os.rollback_monitors.get("test-policy").unwrap().check_regression());
    }

    #[test]
    fn test_serve_os_trading_setup() {
        let mut os = make_serve_os();
        os.setup_trading(TradingConfig::new("localhost", 9876, "S", "T"));
        assert!(os.trading.is_some());
    }

    #[test]
    fn test_serve_os_robotics_setup() {
        let mut os = make_serve_os();
        os.setup_robotics(RoboticsConfig::new(0, 100.0));
        assert!(os.robotics.is_some());
    }

    #[test]
    fn test_serve_os_game_engine_setup() {
        let mut os = make_serve_os();
        os.setup_game_engine(GameEngineConfig::unity(vec![4], 2));
        assert!(os.game_engine.is_some());
    }

    #[test]
    fn test_serve_os_multiple_policies() {
        let mut os = make_serve_os();
        for i in 0..5 {
            let p = PolicyConfig::new(&format!("p{}", i), "v1", 4, 2)
                .with_weights(vec![i as f64; 8], vec![0.0; 2]);
            os.register_policy(p);
        }
        assert_eq!(os.router.policy_count(), 6); // 5 + original
    }

    #[test]
    fn test_serve_os_startup_not_ready() {
        let mut os = RlServeOs::new(100, 60_000);
        os.register_policy(sample_policy(4, 2));
        // Not started yet
        let (_, readiness, _) = os.check_health();
        assert!(!readiness.healthy);
        os.startup();
        let (_, readiness, _) = os.check_health();
        assert!(readiness.healthy);
    }

    // ── Batch Inference tests ─────────────────────────────────────────

    #[test]
    fn test_batch_inference_request() {
        let batch = BatchInferenceRequest::new(
            "b1",
            vec![sample_request(), sample_request()],
            50_000,
        );
        assert_eq!(batch.size(), 2);
    }

    #[test]
    fn test_batch_inference_via_router() {
        let mut router = RequestRouter::new();
        router.register_policy(sample_policy(4, 2));
        let batch = BatchInferenceRequest::new(
            "b1",
            vec![
                InferenceRequest::new("r1", "s1", sample_obs(4)),
                InferenceRequest::new("r2", "s2", sample_obs(4)),
                InferenceRequest::new("r3", "s3", sample_obs(4)),
            ],
            50_000,
        );
        let resp = router.route_batch(&batch);
        assert_eq!(resp.responses.len(), 3);
        assert!(resp.total_latency_us >= 0);
    }

    // ── Replay Entry tests ────────────────────────────────────────────

    #[test]
    fn test_replay_entry() {
        let entry = ReplayEntry::new(
            vec![1.0, 2.0],
            vec![0.0],
            1.0,
            Some(vec![1.5, 2.5]),
            false,
            "p1",
        );
        assert!(!entry.done);
        assert_eq!(entry.policy_id, "p1");
    }

    #[test]
    fn test_replay_entry_terminal() {
        let entry = ReplayEntry::new(vec![1.0], vec![0.0], 0.0, None, true, "p1");
        assert!(entry.done);
        assert!(entry.next_observation.is_none());
    }

    // ── Model Swap Record tests ───────────────────────────────────────

    #[test]
    fn test_model_swap_success() {
        let record = ModelSwapRecord::success("p1", "v1", "v2", 500);
        assert!(record.success);
        assert!(record.error.is_none());
        assert_eq!(record.swap_duration_us, 500);
    }

    #[test]
    fn test_model_swap_failure() {
        let record = ModelSwapRecord::failure("p1", "v1", "v2", "bad weights");
        assert!(!record.success);
        assert_eq!(record.error.as_deref(), Some("bad weights"));
    }

    // ── Health Check Result tests ─────────────────────────────────────

    #[test]
    fn test_health_check_ok() {
        let r = HealthCheckResult::ok(ProbeType::Liveness, 100);
        assert!(r.healthy);
        assert_eq!(r.message, "OK");
    }

    #[test]
    fn test_health_check_fail() {
        let r = HealthCheckResult::fail(ProbeType::Readiness, "not ready");
        assert!(!r.healthy);
        assert_eq!(r.message, "not ready");
    }
}
