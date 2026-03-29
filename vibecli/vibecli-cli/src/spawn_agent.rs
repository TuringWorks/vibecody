//! Parallel agent spawning and lifecycle management — Claude Code-style.
//!
//! Enables spawning autonomous agents from the REPL or chat interface that run
//! in the background, each in an isolated git worktree, with full lifecycle
//! management (start/stop/pause/resume), progress streaming, automatic task
//! decomposition, and intelligent result aggregation.
//!
//! Key concepts:
//! - **SpawnedAgent**: An autonomous agent running a task in its own worktree
//! - **AgentPool**: Manages a pool of running agents with concurrency limits
//! - **TaskDecomposer**: Splits complex tasks into parallel subtasks
//! - **ResultAggregator**: Merges outputs from parallel agents
//! - **AgentBus**: Inter-agent message passing for coordination

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

// ─── Timing helper ──────────────────────────────────────────────────────────

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn short_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = now_millis() & 0xFFFF;
    format!("{:04x}{:04x}", ts, seq & 0xFFFF)
}

// ─── Agent Status ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnStatus {
    /// Queued but not yet started (waiting for pool capacity).
    Queued,
    /// Actively running in a worktree.
    Running,
    /// Temporarily paused — worktree preserved.
    Paused,
    /// Completed successfully.
    Completed,
    /// Failed with an error message.
    Failed,
    /// Cancelled by user.
    Cancelled,
}

impl std::fmt::Display for SpawnStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ─── Agent Priority ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AgentPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}


impl std::fmt::Display for AgentPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ─── Isolation Mode ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum IsolationMode {
    /// Git worktree — separate branch, shared repo.
    #[default]
    Worktree,
    /// Docker container — full isolation.
    Container,
    /// No isolation — runs in current workspace (dangerous for writes).
    None,
}


// ─── Spawn Config ───────────────────────────────────────────────────────────

/// Configuration for spawning an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    /// Human-readable description of the task.
    pub task: String,
    /// Optional short name (auto-generated if absent).
    pub name: Option<String>,
    /// Provider to use (inherits from parent if None).
    pub provider: Option<String>,
    /// Model override.
    pub model: Option<String>,
    /// Maximum agent turns before auto-stop.
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,
    /// Isolation mode.
    #[serde(default)]
    pub isolation: IsolationMode,
    /// Priority level.
    #[serde(default)]
    pub priority: AgentPriority,
    /// Context files to include.
    #[serde(default)]
    pub context_files: Vec<String>,
    /// Whether to run in background (true) or foreground (false).
    #[serde(default = "default_true")]
    pub background: bool,
    /// Approval policy: "suggest" | "auto-edit" | "full-auto".
    #[serde(default = "default_full_auto")]
    pub approval_policy: String,
    /// Optional parent agent ID (for decomposed subtasks).
    pub parent_id: Option<String>,
    /// Tags for grouping/filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Timeout in seconds (0 = no timeout).
    #[serde(default)]
    pub timeout_secs: u64,
}

fn default_max_turns() -> usize { 25 }
fn default_true() -> bool { true }
fn default_full_auto() -> String { "full-auto".to_string() }

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            task: String::new(),
            name: None,
            provider: None,
            model: None,
            max_turns: default_max_turns(),
            isolation: IsolationMode::default(),
            priority: AgentPriority::default(),
            context_files: vec![],
            background: true,
            approval_policy: default_full_auto(),
            parent_id: None,
            tags: vec![],
            timeout_secs: 0,
        }
    }
}

impl SpawnConfig {
    pub fn new(task: impl Into<String>) -> Self {
        Self { task: task.into(), ..Default::default() }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_provider(mut self, p: impl Into<String>) -> Self {
        self.provider = Some(p.into());
        self
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }

    pub fn with_priority(mut self, p: AgentPriority) -> Self {
        self.priority = p;
        self
    }

    pub fn with_isolation(mut self, mode: IsolationMode) -> Self {
        self.isolation = mode;
        self
    }

    pub fn with_context_files(mut self, files: Vec<String>) -> Self {
        self.context_files = files;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_max_turns(mut self, n: usize) -> Self {
        self.max_turns = n;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn foreground(mut self) -> Self {
        self.background = false;
        self
    }
}

// ─── Agent Progress ─────────────────────────────────────────────────────────

/// Tracks progress of a spawned agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProgress {
    /// Completed turns so far.
    pub turns_completed: usize,
    /// Total turns allowed.
    pub turns_limit: usize,
    /// Files modified so far.
    pub files_modified: Vec<String>,
    /// Latest status message from the agent.
    pub last_message: Option<String>,
    /// Tool calls made.
    pub tool_calls: usize,
    /// Tokens consumed.
    pub tokens_used: u64,
    /// Estimated percentage complete (0-100).
    pub percent_complete: u8,
}

impl Default for AgentProgress {
    fn default() -> Self {
        Self {
            turns_completed: 0,
            turns_limit: 25,
            files_modified: vec![],
            last_message: None,
            tool_calls: 0,
            tokens_used: 0,
            percent_complete: 0,
        }
    }
}

impl AgentProgress {
    pub fn new(turns_limit: usize) -> Self {
        Self { turns_limit, ..Default::default() }
    }

    /// Update percent based on turns.
    pub fn update_percent(&mut self) {
        if self.turns_limit > 0 {
            let pct = (self.turns_completed as f64 / self.turns_limit as f64 * 100.0).min(99.0);
            self.percent_complete = pct as u8;
        }
    }

    pub fn record_turn(&mut self, message: Option<String>, files: &[String], tool_call_count: usize, tokens: u64) {
        self.turns_completed += 1;
        if let Some(msg) = message {
            self.last_message = Some(msg);
        }
        for f in files {
            if !self.files_modified.contains(f) {
                self.files_modified.push(f.clone());
            }
        }
        self.tool_calls += tool_call_count;
        self.tokens_used += tokens;
        self.update_percent();
    }

    pub fn finish(&mut self) {
        self.percent_complete = 100;
    }
}

// ─── SpawnedAgent ───────────────────────────────────────────────────────────

/// A spawned autonomous agent with full lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedAgent {
    /// Unique ID (e.g., "sa_1a2b3c4d").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The task description.
    pub task: String,
    /// Current status.
    pub status: SpawnStatus,
    /// Configuration used to spawn.
    pub config: SpawnConfig,
    /// Progress tracking.
    pub progress: AgentProgress,
    /// Git branch (if worktree isolation).
    pub branch: Option<String>,
    /// Worktree path (if applicable).
    pub worktree_path: Option<PathBuf>,
    /// Result summary (set on completion).
    pub result_summary: Option<String>,
    /// Error message (set on failure).
    pub error: Option<String>,
    /// When the agent was created (unix ms).
    pub created_at: u64,
    /// When the agent started running (unix ms).
    pub started_at: Option<u64>,
    /// When the agent finished (unix ms).
    pub finished_at: Option<u64>,
    /// Parent agent ID (for subtasks).
    pub parent_id: Option<String>,
    /// Child agent IDs (subtasks spawned by this agent).
    pub child_ids: Vec<String>,
    /// Messages received from other agents.
    pub inbox: Vec<AgentMessage>,
}

impl SpawnedAgent {
    pub fn new(config: SpawnConfig) -> Self {
        let id = format!("sa_{}", short_id());
        let name = config.name.clone().unwrap_or_else(|| {
            let words: Vec<&str> = config.task.split_whitespace().take(4).collect();
            if words.is_empty() { "agent".to_string() } else { words.join("-").to_lowercase() }
        });
        let branch = match config.isolation {
            IsolationMode::Worktree => Some(format!("spawn-{}", &id)),
            _ => None,
        };
        let turns_limit = config.max_turns;
        Self {
            id,
            name,
            task: config.task.clone(),
            status: SpawnStatus::Queued,
            config,
            progress: AgentProgress::new(turns_limit),
            branch,
            worktree_path: None,
            result_summary: None,
            error: None,
            created_at: now_millis(),
            started_at: None,
            finished_at: None,
            parent_id: None,
            child_ids: vec![],
            inbox: vec![],
        }
    }

    pub fn start(&mut self) {
        self.status = SpawnStatus::Running;
        self.started_at = Some(now_millis());
    }

    pub fn pause(&mut self) {
        if self.status == SpawnStatus::Running {
            self.status = SpawnStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == SpawnStatus::Paused {
            self.status = SpawnStatus::Running;
        }
    }

    pub fn complete(&mut self, summary: String) {
        self.status = SpawnStatus::Completed;
        self.finished_at = Some(now_millis());
        self.result_summary = Some(summary);
        self.progress.finish();
    }

    pub fn fail(&mut self, error: String) {
        self.status = SpawnStatus::Failed;
        self.finished_at = Some(now_millis());
        self.error = Some(error);
    }

    pub fn cancel(&mut self) {
        self.status = SpawnStatus::Cancelled;
        self.finished_at = Some(now_millis());
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SpawnStatus::Running | SpawnStatus::Paused | SpawnStatus::Queued)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status, SpawnStatus::Completed | SpawnStatus::Failed | SpawnStatus::Cancelled)
    }

    pub fn duration_ms(&self) -> u64 {
        let start = self.started_at.unwrap_or(self.created_at);
        let end = self.finished_at.unwrap_or_else(now_millis);
        end.saturating_sub(start)
    }

    pub fn duration_human(&self) -> String {
        let ms = self.duration_ms();
        if ms < 1000 {
            format!("{}ms", ms)
        } else if ms < 60_000 {
            format!("{:.1}s", ms as f64 / 1000.0)
        } else {
            let mins = ms / 60_000;
            let secs = (ms % 60_000) / 1000;
            format!("{}m{}s", mins, secs)
        }
    }

    pub fn receive_message(&mut self, msg: AgentMessage) {
        self.inbox.push(msg);
    }
}

// ─── Inter-Agent Messaging ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from_id: String,
    pub to_id: String,
    pub msg_type: MessageType,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Informational status update.
    Status,
    /// Request for help / delegation.
    Request,
    /// Response to a request.
    Response,
    /// File changed notification.
    FileChange,
    /// Conflict detected.
    Conflict,
    /// Task completed notification.
    Done,
}

impl AgentMessage {
    pub fn new(from: impl Into<String>, to: impl Into<String>, msg_type: MessageType, content: impl Into<String>) -> Self {
        Self {
            from_id: from.into(),
            to_id: to.into(),
            msg_type,
            content: content.into(),
            timestamp: now_millis(),
        }
    }
}

// ─── Task Decomposition ─────────────────────────────────────────────────────

/// Strategy for decomposing a complex task into parallel subtasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecomposeStrategy {
    /// File-based: one agent per file/directory.
    ByFile,
    /// Concern-based: separate agents for tests, docs, implementation.
    ByConcern,
    /// Component-based: one agent per logical component.
    ByComponent,
    /// Custom: user provides explicit subtask list.
    Custom,
}

/// A decomposed subtask ready to be spawned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub description: String,
    pub files: Vec<String>,
    pub depends_on: Vec<usize>,
    pub priority: AgentPriority,
}

/// Decomposes a task into parallelizable subtasks.
pub struct TaskDecomposer;

impl TaskDecomposer {
    /// Analyze a task description and produce subtasks using the given strategy.
    pub fn decompose(task: &str, strategy: &DecomposeStrategy, context_files: &[String]) -> Vec<SubTask> {
        match strategy {
            DecomposeStrategy::ByConcern => Self::decompose_by_concern(task),
            DecomposeStrategy::ByFile => Self::decompose_by_file(task, context_files),
            DecomposeStrategy::ByComponent => Self::decompose_by_component(task, context_files),
            DecomposeStrategy::Custom => vec![SubTask {
                description: task.to_string(),
                files: context_files.to_vec(),
                depends_on: vec![],
                priority: AgentPriority::Normal,
            }],
        }
    }

    fn decompose_by_concern(task: &str) -> Vec<SubTask> {
        let mut subtasks = vec![];

        // Implementation subtask
        subtasks.push(SubTask {
            description: format!("Implement: {}", task),
            files: vec![],
            depends_on: vec![],
            priority: AgentPriority::High,
        });

        // Test writing subtask (depends on implementation)
        subtasks.push(SubTask {
            description: format!("Write tests for: {}", task),
            files: vec![],
            depends_on: vec![0],
            priority: AgentPriority::Normal,
        });

        // Documentation subtask (depends on implementation)
        subtasks.push(SubTask {
            description: format!("Write documentation for: {}", task),
            files: vec![],
            depends_on: vec![0],
            priority: AgentPriority::Low,
        });

        subtasks
    }

    fn decompose_by_file(_task: &str, context_files: &[String]) -> Vec<SubTask> {
        if context_files.is_empty() {
            return vec![SubTask {
                description: _task.to_string(),
                files: vec![],
                depends_on: vec![],
                priority: AgentPriority::Normal,
            }];
        }

        context_files
            .iter()
            .map(|file| SubTask {
                description: format!("Process {}: {}", file, _task),
                files: vec![file.clone()],
                depends_on: vec![],
                priority: AgentPriority::Normal,
            })
            .collect()
    }

    fn decompose_by_component(_task: &str, context_files: &[String]) -> Vec<SubTask> {
        // Group files by directory (each dir = one component)
        let mut components: HashMap<String, Vec<String>> = HashMap::new();
        for file in context_files {
            let dir = std::path::Path::new(file)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("root")
                .to_string();
            components.entry(dir).or_default().push(file.clone());
        }

        if components.is_empty() {
            return vec![SubTask {
                description: _task.to_string(),
                files: vec![],
                depends_on: vec![],
                priority: AgentPriority::Normal,
            }];
        }

        components
            .into_iter()
            .map(|(dir, files)| SubTask {
                description: format!("Handle component {}: {}", dir, _task),
                files,
                depends_on: vec![],
                priority: AgentPriority::Normal,
            })
            .collect()
    }
}

// ─── Result Aggregation ─────────────────────────────────────────────────────

/// Strategy for merging results from multiple parallel agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Pick the single best result (by score/success).
    BestResult,
    /// Merge all branches sequentially.
    SequentialMerge,
    /// Cherry-pick specific commits from each branch.
    CherryPick,
    /// Let the user decide.
    Manual,
}

/// Result from aggregating multiple agent outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResult {
    pub strategy: MergeStrategy,
    pub total_agents: usize,
    pub successful_agents: usize,
    pub failed_agents: usize,
    pub best_agent_id: Option<String>,
    pub merged_branch: Option<String>,
    pub summaries: Vec<AgentSummary>,
    pub conflicts: Vec<MergeConflict>,
    pub total_files_modified: usize,
    pub total_tokens_used: u64,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub agent_name: String,
    pub status: SpawnStatus,
    pub summary: Option<String>,
    pub files_modified: usize,
    pub turns_taken: usize,
    pub tokens_used: u64,
    pub duration_ms: u64,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflict {
    pub file: String,
    pub agent_a: String,
    pub agent_b: String,
    pub description: String,
}

/// Aggregates results from multiple spawned agents.
pub struct ResultAggregator;

impl ResultAggregator {
    /// Aggregate results from a list of completed agents.
    pub fn aggregate(agents: &[SpawnedAgent], strategy: &MergeStrategy) -> AggregatedResult {
        let summaries: Vec<AgentSummary> = agents
            .iter()
            .map(|a| AgentSummary {
                agent_id: a.id.clone(),
                agent_name: a.name.clone(),
                status: a.status.clone(),
                summary: a.result_summary.clone(),
                files_modified: a.progress.files_modified.len(),
                turns_taken: a.progress.turns_completed,
                tokens_used: a.progress.tokens_used,
                duration_ms: a.duration_ms(),
                branch: a.branch.clone(),
            })
            .collect();

        let successful: Vec<&SpawnedAgent> = agents
            .iter()
            .filter(|a| a.status == SpawnStatus::Completed)
            .collect();

        let best_agent_id = match strategy {
            MergeStrategy::BestResult => {
                // Pick agent with most files modified + fewest turns (efficient)
                successful
                    .iter()
                    .max_by_key(|a| {
                        let file_score = a.progress.files_modified.len() * 100;
                        let efficiency = if a.progress.turns_completed > 0 {
                            100 / a.progress.turns_completed
                        } else {
                            0
                        };
                        file_score + efficiency
                    })
                    .map(|a| a.id.clone())
            }
            _ => successful.first().map(|a| a.id.clone()),
        };

        // Detect file conflicts across agents
        let conflicts = Self::detect_conflicts(agents);

        let total_files: std::collections::HashSet<&String> = agents
            .iter()
            .flat_map(|a| a.progress.files_modified.iter())
            .collect();

        AggregatedResult {
            strategy: strategy.clone(),
            total_agents: agents.len(),
            successful_agents: successful.len(),
            failed_agents: agents.iter().filter(|a| a.status == SpawnStatus::Failed).count(),
            best_agent_id,
            merged_branch: None,
            summaries,
            conflicts,
            total_files_modified: total_files.len(),
            total_tokens_used: agents.iter().map(|a| a.progress.tokens_used).sum(),
            total_duration_ms: agents.iter().map(|a| a.duration_ms()).max().unwrap_or(0),
        }
    }

    fn detect_conflicts(agents: &[SpawnedAgent]) -> Vec<MergeConflict> {
        let mut file_owners: HashMap<&String, Vec<&SpawnedAgent>> = HashMap::new();
        for agent in agents {
            for file in &agent.progress.files_modified {
                file_owners.entry(file).or_default().push(agent);
            }
        }

        let mut conflicts = vec![];
        for (file, owners) in &file_owners {
            if owners.len() > 1 {
                for i in 0..owners.len() {
                    for j in (i + 1)..owners.len() {
                        conflicts.push(MergeConflict {
                            file: (*file).clone(),
                            agent_a: owners[i].id.clone(),
                            agent_b: owners[j].id.clone(),
                            description: format!(
                                "Both {} and {} modified {}",
                                owners[i].name, owners[j].name, file
                            ),
                        });
                    }
                }
            }
        }
        conflicts
    }
}

// ─── Agent Pool ─────────────────────────────────────────────────────────────

/// Pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of agents running concurrently.
    pub max_concurrent: usize,
    /// Maximum total agents in the pool (including queued).
    pub max_total: usize,
    /// Default merge strategy for parallel runs.
    pub default_merge_strategy: MergeStrategy,
    /// Auto-cleanup completed agents after this many seconds (0 = keep forever).
    pub auto_cleanup_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_total: 20,
            default_merge_strategy: MergeStrategy::BestResult,
            auto_cleanup_secs: 3600,
        }
    }
}

/// Manages a pool of spawned agents.
pub struct AgentPool {
    agents: Arc<Mutex<HashMap<String, SpawnedAgent>>>,
    config: PoolConfig,
    /// Queue of agent IDs ordered by priority then creation time.
    queue: Arc<Mutex<Vec<String>>>,
    /// Messages waiting to be delivered.
    message_bus: Arc<Mutex<Vec<AgentMessage>>>,
}

impl AgentPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            config,
            queue: Arc::new(Mutex::new(Vec::new())),
            message_bus: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PoolConfig::default())
    }

    /// Spawn a new agent. Returns the agent ID.
    pub fn spawn(&self, config: SpawnConfig) -> Result<String, String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        if agents.len() >= self.config.max_total {
            return Err(format!(
                "Pool is full ({}/{}). Cancel or wait for agents to finish.",
                agents.len(),
                self.config.max_total
            ));
        }

        let mut agent = SpawnedAgent::new(config);
        let id = agent.id.clone();

        // Check if we can start immediately or queue (atomic under same lock)
        let running_count = agents.values().filter(|a| a.status == SpawnStatus::Running).count();
        if running_count < self.config.max_concurrent {
            agent.start();
        } else {
            // Queue by priority
            let mut queue = self.queue.lock().map_err(|e| format!("Lock error: {}", e))?;
            queue.push(id.clone());
        }

        agents.insert(id.clone(), agent);
        Ok(id)
    }

    /// Spawn multiple agents for a decomposed task. Returns parent ID and child IDs.
    pub fn spawn_decomposed(
        &self,
        parent_task: &str,
        strategy: &DecomposeStrategy,
        context_files: &[String],
        base_config: &SpawnConfig,
    ) -> Result<(String, Vec<String>), String> {
        let subtasks = TaskDecomposer::decompose(parent_task, strategy, context_files);

        // Create parent agent
        let parent_config = SpawnConfig::new(format!("[coordinator] {}", parent_task))
            .with_name(format!("coordinator-{}", short_id()))
            .with_priority(AgentPriority::High);
        let parent_id = self.spawn(parent_config)?;

        // Create child agents
        let mut child_ids = vec![];
        for subtask in &subtasks {
            let mut child_config = base_config.clone();
            child_config.task = subtask.description.clone();
            child_config.context_files = subtask.files.clone();
            child_config.priority = subtask.priority;
            child_config.parent_id = Some(parent_id.clone());

            match self.spawn(child_config) {
                Ok(child_id) => child_ids.push(child_id),
                Err(e) => {
                    // If we can't spawn all children, log but continue
                    eprintln!("Warning: failed to spawn subtask: {}", e);
                }
            }
        }

        // Link parent to children
        if let Ok(mut agents) = self.agents.lock() {
            if let Some(parent) = agents.get_mut(&parent_id) {
                parent.child_ids = child_ids.clone();
            }
        }

        Ok((parent_id, child_ids))
    }

    /// Get agent by ID.
    pub fn get(&self, id: &str) -> Option<SpawnedAgent> {
        self.agents.lock().ok()?.get(id).cloned()
    }

    /// List all agents, optionally filtered by status.
    pub fn list(&self, status_filter: Option<&SpawnStatus>) -> Vec<SpawnedAgent> {
        let agents = match self.agents.lock() {
            Ok(a) => a,
            Err(_) => return vec![],
        };
        let mut result: Vec<SpawnedAgent> = agents
            .values()
            .filter(|a| status_filter.is_none() || status_filter == Some(&a.status))
            .cloned()
            .collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result
    }

    /// List active (non-terminal) agents.
    pub fn list_active(&self) -> Vec<SpawnedAgent> {
        let agents = match self.agents.lock() {
            Ok(a) => a,
            Err(_) => return vec![],
        };
        let mut result: Vec<SpawnedAgent> = agents
            .values()
            .filter(|a| a.is_active())
            .cloned()
            .collect();
        result.sort_by(|a, b| b.config.priority.cmp(&a.config.priority));
        result
    }

    /// Count running agents.
    pub fn running_count(&self) -> usize {
        self.agents
            .lock()
            .map(|a| a.values().filter(|ag| ag.status == SpawnStatus::Running).count())
            .unwrap_or(0)
    }

    /// Pause an agent.
    pub fn pause(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        if agent.status != SpawnStatus::Running {
            return Err(format!("Cannot pause agent in {} state", agent.status));
        }
        agent.pause();
        Ok(())
    }

    /// Resume a paused agent.
    pub fn resume(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        if agent.status != SpawnStatus::Paused {
            return Err(format!("Cannot resume agent in {} state", agent.status));
        }
        agent.resume();
        Ok(())
    }

    /// Cancel an agent.
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        if agent.is_terminal() {
            return Err(format!("Agent already in terminal state: {}", agent.status));
        }
        agent.cancel();
        drop(agents);
        self.promote_queued();
        Ok(())
    }

    /// Complete an agent with a summary.
    pub fn complete(&self, id: &str, summary: String) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        agent.complete(summary);
        drop(agents);
        self.promote_queued();
        Ok(())
    }

    /// Mark an agent as failed.
    pub fn fail(&self, id: &str, error: String) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        agent.fail(error);
        drop(agents);
        self.promote_queued();
        Ok(())
    }

    /// Record a progress update for an agent.
    pub fn update_progress(
        &self,
        id: &str,
        message: Option<String>,
        files: &[String],
        tool_calls: usize,
        tokens: u64,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let agent = agents.get_mut(id).ok_or_else(|| format!("Agent not found: {}", id))?;
        agent.progress.record_turn(message, files, tool_calls, tokens);
        Ok(())
    }

    /// Send a message between agents.
    pub fn send_message(&self, msg: AgentMessage) -> Result<(), String> {
        // Deliver directly to target agent's inbox
        let mut agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        if let Some(target) = agents.get_mut(&msg.to_id) {
            target.receive_message(msg);
            Ok(())
        } else {
            // Queue for later delivery
            let mut bus = self.message_bus.lock().map_err(|e| format!("Lock error: {}", e))?;
            bus.push(msg);
            Ok(())
        }
    }

    /// Promote queued agents to running when capacity is available.
    fn promote_queued(&self) {
        let running = self.running_count();
        if running >= self.config.max_concurrent {
            return;
        }

        let slots = self.config.max_concurrent - running;
        let mut queue = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return,
        };
        let mut agents = match self.agents.lock() {
            Ok(a) => a,
            Err(_) => return,
        };

        // Sort queue by priority
        queue.sort_by(|a_id, b_id| {
            let a_pri = agents.get(a_id).map(|a| a.config.priority).unwrap_or(AgentPriority::Low);
            let b_pri = agents.get(b_id).map(|a| a.config.priority).unwrap_or(AgentPriority::Low);
            b_pri.cmp(&a_pri)
        });

        let count = slots.min(queue.len());
        let to_promote: Vec<String> = queue.drain(..count).collect();
        for id in to_promote {
            if let Some(agent) = agents.get_mut(&id) {
                agent.start();
            }
        }
    }

    /// Aggregate results from all agents in a decomposed task.
    pub fn aggregate_results(&self, parent_id: &str) -> Result<AggregatedResult, String> {
        let agents = self.agents.lock().map_err(|e| format!("Lock error: {}", e))?;
        let parent = agents.get(parent_id).ok_or_else(|| format!("Agent not found: {}", parent_id))?;

        let children: Vec<SpawnedAgent> = parent
            .child_ids
            .iter()
            .filter_map(|id| agents.get(id).cloned())
            .collect();

        if children.is_empty() {
            return Err("No child agents found".to_string());
        }

        Ok(ResultAggregator::aggregate(&children, &self.config.default_merge_strategy))
    }

    /// Cleanup completed agents older than `max_age_ms`.
    pub fn cleanup(&self, max_age_ms: u64) -> usize {
        let now = now_millis();
        let mut agents = match self.agents.lock() {
            Ok(a) => a,
            Err(_) => return 0,
        };
        let before = agents.len();
        agents.retain(|_, a| {
            if a.is_terminal() {
                let age = now.saturating_sub(a.finished_at.unwrap_or(a.created_at));
                age < max_age_ms
            } else {
                true
            }
        });
        before - agents.len()
    }

    /// Get pool statistics.
    pub fn stats(&self) -> PoolStats {
        let agents = match self.agents.lock() {
            Ok(a) => a,
            Err(_) => return PoolStats::default(),
        };

        let mut stats = PoolStats {
            total: agents.len(),
            max_concurrent: self.config.max_concurrent,
            max_total: self.config.max_total,
            ..Default::default()
        };

        for agent in agents.values() {
            match agent.status {
                SpawnStatus::Queued => stats.queued += 1,
                SpawnStatus::Running => stats.running += 1,
                SpawnStatus::Paused => stats.paused += 1,
                SpawnStatus::Completed => stats.completed += 1,
                SpawnStatus::Failed => stats.failed += 1,
                SpawnStatus::Cancelled => stats.cancelled += 1,
            }
            stats.total_tokens += agent.progress.tokens_used;
            stats.total_files_modified += agent.progress.files_modified.len();
        }

        stats
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoolStats {
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub paused: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub max_concurrent: usize,
    pub max_total: usize,
    pub total_tokens: u64,
    pub total_files_modified: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool: {}/{} total, {} running, {} queued, {} paused, {} done, {} failed, {} cancelled | tokens: {} | files: {}",
            self.total,
            self.max_total,
            self.running,
            self.queued,
            self.paused,
            self.completed,
            self.failed,
            self.cancelled,
            self.total_tokens,
            self.total_files_modified,
        )
    }
}

// ─── Global Pool Singleton ──────────────────────────────────────────────────

/// Returns a shared global agent pool for the REPL session.
pub fn global_pool() -> &'static AgentPool {
    use std::sync::OnceLock;
    static POOL: OnceLock<AgentPool> = OnceLock::new();
    POOL.get_or_init(AgentPool::with_defaults)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpawnConfig ─────────────────────────────────────────────────────

    #[test]
    fn spawn_config_defaults() {
        let cfg = SpawnConfig::default();
        assert_eq!(cfg.max_turns, 25);
        assert_eq!(cfg.isolation, IsolationMode::Worktree);
        assert_eq!(cfg.priority, AgentPriority::Normal);
        assert!(cfg.background);
        assert_eq!(cfg.approval_policy, "full-auto");
        assert!(cfg.context_files.is_empty());
        assert!(cfg.tags.is_empty());
    }

    #[test]
    fn spawn_config_builder() {
        let cfg = SpawnConfig::new("fix bug")
            .with_name("bugfix")
            .with_provider("claude")
            .with_model("opus")
            .with_priority(AgentPriority::Critical)
            .with_isolation(IsolationMode::Container)
            .with_context_files(vec!["main.rs".into()])
            .with_tags(vec!["urgent".into()])
            .with_max_turns(50)
            .with_timeout(300)
            .foreground();

        assert_eq!(cfg.task, "fix bug");
        assert_eq!(cfg.name.as_deref(), Some("bugfix"));
        assert_eq!(cfg.provider.as_deref(), Some("claude"));
        assert_eq!(cfg.model.as_deref(), Some("opus"));
        assert_eq!(cfg.priority, AgentPriority::Critical);
        assert_eq!(cfg.isolation, IsolationMode::Container);
        assert_eq!(cfg.context_files, vec!["main.rs"]);
        assert_eq!(cfg.tags, vec!["urgent"]);
        assert_eq!(cfg.max_turns, 50);
        assert_eq!(cfg.timeout_secs, 300);
        assert!(!cfg.background);
    }

    // ── SpawnedAgent ────────────────────────────────────────────────────

    #[test]
    fn spawned_agent_lifecycle() {
        let cfg = SpawnConfig::new("test task").with_name("test-agent");
        let mut agent = SpawnedAgent::new(cfg);

        assert!(agent.id.starts_with("sa_"));
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.status, SpawnStatus::Queued);
        assert!(agent.is_active());
        assert!(!agent.is_terminal());

        agent.start();
        assert_eq!(agent.status, SpawnStatus::Running);
        assert!(agent.started_at.is_some());

        agent.pause();
        assert_eq!(agent.status, SpawnStatus::Paused);

        agent.resume();
        assert_eq!(agent.status, SpawnStatus::Running);

        agent.complete("Done!".to_string());
        assert_eq!(agent.status, SpawnStatus::Completed);
        assert!(agent.is_terminal());
        assert!(!agent.is_active());
        assert_eq!(agent.result_summary.as_deref(), Some("Done!"));
        assert_eq!(agent.progress.percent_complete, 100);
    }

    #[test]
    fn spawned_agent_fail() {
        let mut agent = SpawnedAgent::new(SpawnConfig::new("will fail"));
        agent.start();
        agent.fail("out of memory".to_string());
        assert_eq!(agent.status, SpawnStatus::Failed);
        assert_eq!(agent.error.as_deref(), Some("out of memory"));
        assert!(agent.is_terminal());
    }

    #[test]
    fn spawned_agent_cancel() {
        let mut agent = SpawnedAgent::new(SpawnConfig::new("cancel me"));
        agent.start();
        agent.cancel();
        assert_eq!(agent.status, SpawnStatus::Cancelled);
        assert!(agent.is_terminal());
    }

    #[test]
    fn spawned_agent_auto_name_from_task() {
        let agent = SpawnedAgent::new(SpawnConfig::new("fix authentication bug in login"));
        assert_eq!(agent.name, "fix-authentication-bug-in");
    }

    #[test]
    fn spawned_agent_worktree_branch() {
        let agent = SpawnedAgent::new(SpawnConfig::new("test").with_isolation(IsolationMode::Worktree));
        assert!(agent.branch.as_ref().expect("should have branch").starts_with("spawn-sa_"));

        let agent2 = SpawnedAgent::new(SpawnConfig::new("test").with_isolation(IsolationMode::Container));
        assert!(agent2.branch.is_none());
    }

    #[test]
    fn spawned_agent_messaging() {
        let mut agent = SpawnedAgent::new(SpawnConfig::new("test"));
        assert!(agent.inbox.is_empty());
        let msg = AgentMessage::new("other", &agent.id, MessageType::Status, "hello");
        agent.receive_message(msg);
        assert_eq!(agent.inbox.len(), 1);
        assert_eq!(agent.inbox[0].content, "hello");
    }

    #[test]
    fn spawned_agent_duration_human() {
        let mut agent = SpawnedAgent::new(SpawnConfig::new("test"));
        agent.started_at = Some(now_millis() - 125_000);
        agent.finished_at = Some(now_millis());
        let dur = agent.duration_human();
        assert!(dur.contains("m"), "Expected minutes format, got: {}", dur);
    }

    // ── AgentProgress ───────────────────────────────────────────────────

    #[test]
    fn progress_tracking() {
        let mut p = AgentProgress::new(10);
        assert_eq!(p.percent_complete, 0);
        p.record_turn(Some("step 1".into()), &["a.rs".into()], 2, 500);
        assert_eq!(p.turns_completed, 1);
        assert_eq!(p.tool_calls, 2);
        assert_eq!(p.tokens_used, 500);
        assert_eq!(p.files_modified, vec!["a.rs"]);
        assert_eq!(p.percent_complete, 10);

        // Duplicate file doesn't add again
        p.record_turn(None, &["a.rs".into(), "b.rs".into()], 1, 300);
        assert_eq!(p.files_modified.len(), 2);
        assert_eq!(p.tokens_used, 800);
        assert_eq!(p.turns_completed, 2);

        p.finish();
        assert_eq!(p.percent_complete, 100);
    }

    // ── AgentMessage ────────────────────────────────────────────────────

    #[test]
    fn message_creation() {
        let msg = AgentMessage::new("a1", "a2", MessageType::FileChange, "modified main.rs");
        assert_eq!(msg.from_id, "a1");
        assert_eq!(msg.to_id, "a2");
        assert_eq!(msg.msg_type, MessageType::FileChange);
        assert_eq!(msg.content, "modified main.rs");
        assert!(msg.timestamp > 0);
    }

    // ── TaskDecomposer ──────────────────────────────────────────────────

    #[test]
    fn decompose_by_concern() {
        let subtasks = TaskDecomposer::decompose("add auth", &DecomposeStrategy::ByConcern, &[]);
        assert_eq!(subtasks.len(), 3);
        assert!(subtasks[0].description.contains("Implement"));
        assert!(subtasks[1].description.contains("tests"));
        assert!(subtasks[2].description.contains("documentation"));
        assert_eq!(subtasks[1].depends_on, vec![0]);
        assert_eq!(subtasks[2].depends_on, vec![0]);
    }

    #[test]
    fn decompose_by_file() {
        let files = vec!["a.rs".into(), "b.rs".into(), "c.rs".into()];
        let subtasks = TaskDecomposer::decompose("lint", &DecomposeStrategy::ByFile, &files);
        assert_eq!(subtasks.len(), 3);
        assert_eq!(subtasks[0].files, vec!["a.rs"]);
        assert_eq!(subtasks[1].files, vec!["b.rs"]);
        assert!(subtasks[0].depends_on.is_empty());
    }

    #[test]
    fn decompose_by_file_empty() {
        let subtasks = TaskDecomposer::decompose("lint", &DecomposeStrategy::ByFile, &[]);
        assert_eq!(subtasks.len(), 1);
    }

    #[test]
    fn decompose_by_component() {
        let files = vec!["src/auth/login.rs".into(), "src/auth/session.rs".into(), "src/api/handler.rs".into()];
        let subtasks = TaskDecomposer::decompose("refactor", &DecomposeStrategy::ByComponent, &files);
        assert_eq!(subtasks.len(), 2); // two directories
    }

    #[test]
    fn decompose_custom() {
        let subtasks = TaskDecomposer::decompose("do thing", &DecomposeStrategy::Custom, &["x.rs".into()]);
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].description, "do thing");
    }

    // ── ResultAggregator ────────────────────────────────────────────────

    #[test]
    fn aggregate_results_basic() {
        let mut a1 = SpawnedAgent::new(SpawnConfig::new("task A"));
        a1.start();
        a1.progress.record_turn(None, &["file1.rs".into()], 3, 1000);
        a1.complete("Did A".to_string());

        let mut a2 = SpawnedAgent::new(SpawnConfig::new("task B"));
        a2.start();
        a2.progress.record_turn(None, &["file2.rs".into(), "file3.rs".into()], 5, 2000);
        a2.complete("Did B".to_string());

        let result = ResultAggregator::aggregate(&[a1, a2], &MergeStrategy::BestResult);
        assert_eq!(result.total_agents, 2);
        assert_eq!(result.successful_agents, 2);
        assert_eq!(result.failed_agents, 0);
        assert_eq!(result.total_files_modified, 3);
        assert_eq!(result.total_tokens_used, 3000);
        assert!(result.best_agent_id.is_some());
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn aggregate_detects_conflicts() {
        let mut a1 = SpawnedAgent::new(SpawnConfig::new("task A").with_name("agent-a"));
        a1.start();
        a1.progress.record_turn(None, &["shared.rs".into()], 1, 100);
        a1.complete("Done".to_string());

        let mut a2 = SpawnedAgent::new(SpawnConfig::new("task B").with_name("agent-b"));
        a2.start();
        a2.progress.record_turn(None, &["shared.rs".into()], 1, 100);
        a2.complete("Done".to_string());

        let result = ResultAggregator::aggregate(&[a1, a2], &MergeStrategy::BestResult);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].file, "shared.rs");
    }

    #[test]
    fn aggregate_with_failures() {
        let mut a1 = SpawnedAgent::new(SpawnConfig::new("ok"));
        a1.start();
        a1.complete("Done".to_string());

        let mut a2 = SpawnedAgent::new(SpawnConfig::new("fail"));
        a2.start();
        a2.fail("error".to_string());

        let result = ResultAggregator::aggregate(&[a1, a2], &MergeStrategy::BestResult);
        assert_eq!(result.successful_agents, 1);
        assert_eq!(result.failed_agents, 1);
    }

    // ── AgentPool ───────────────────────────────────────────────────────

    #[test]
    fn pool_spawn_and_list() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 2, max_total: 5, ..Default::default() });

        let id1 = pool.spawn(SpawnConfig::new("task 1")).expect("spawn 1");
        let id2 = pool.spawn(SpawnConfig::new("task 2")).expect("spawn 2");
        let id3 = pool.spawn(SpawnConfig::new("task 3")).expect("spawn 3");

        // First 2 should be running, 3rd queued
        let a1 = pool.get(&id1).expect("get 1");
        let a2 = pool.get(&id2).expect("get 2");
        let a3 = pool.get(&id3).expect("get 3");
        assert_eq!(a1.status, SpawnStatus::Running);
        assert_eq!(a2.status, SpawnStatus::Running);
        assert_eq!(a3.status, SpawnStatus::Queued);

        assert_eq!(pool.running_count(), 2);
        assert_eq!(pool.list(None).len(), 3);
    }

    #[test]
    fn pool_max_total_enforced() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 2, ..Default::default() });
        pool.spawn(SpawnConfig::new("a")).expect("spawn a");
        pool.spawn(SpawnConfig::new("b")).expect("spawn b");
        let result = pool.spawn(SpawnConfig::new("c"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Pool is full"));
    }

    #[test]
    fn pool_pause_resume() {
        let pool = AgentPool::with_defaults();
        let id = pool.spawn(SpawnConfig::new("pausable")).expect("spawn");

        pool.pause(&id).expect("pause");
        assert_eq!(pool.get(&id).unwrap().status, SpawnStatus::Paused);

        pool.resume(&id).expect("resume");
        assert_eq!(pool.get(&id).unwrap().status, SpawnStatus::Running);
    }

    #[test]
    fn pool_cancel_promotes_queued() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 5, ..Default::default() });
        let id1 = pool.spawn(SpawnConfig::new("running")).expect("spawn 1");
        let id2 = pool.spawn(SpawnConfig::new("queued")).expect("spawn 2");

        assert_eq!(pool.get(&id2).unwrap().status, SpawnStatus::Queued);
        pool.cancel(&id1).expect("cancel");
        // After cancel, queued agent should be promoted
        assert_eq!(pool.get(&id2).unwrap().status, SpawnStatus::Running);
    }

    #[test]
    fn pool_complete_promotes_queued() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 5, ..Default::default() });
        let id1 = pool.spawn(SpawnConfig::new("will complete")).expect("spawn 1");
        let id2 = pool.spawn(SpawnConfig::new("waiting")).expect("spawn 2");

        pool.complete(&id1, "done".to_string()).expect("complete");
        assert_eq!(pool.get(&id2).unwrap().status, SpawnStatus::Running);
    }

    #[test]
    fn pool_fail_promotes_queued() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 5, ..Default::default() });
        let id1 = pool.spawn(SpawnConfig::new("will fail")).expect("spawn 1");
        let id2 = pool.spawn(SpawnConfig::new("waiting")).expect("spawn 2");

        pool.fail(&id1, "oops".to_string()).expect("fail");
        assert_eq!(pool.get(&id2).unwrap().status, SpawnStatus::Running);
    }

    #[test]
    fn pool_update_progress() {
        let pool = AgentPool::with_defaults();
        let id = pool.spawn(SpawnConfig::new("progress test")).expect("spawn");

        pool.update_progress(&id, Some("working...".into()), &["a.rs".into()], 2, 500).expect("update");
        let agent = pool.get(&id).unwrap();
        assert_eq!(agent.progress.turns_completed, 1);
        assert_eq!(agent.progress.tokens_used, 500);
        assert_eq!(agent.progress.files_modified, vec!["a.rs"]);
    }

    #[test]
    fn pool_send_message() {
        let pool = AgentPool::with_defaults();
        let id1 = pool.spawn(SpawnConfig::new("sender")).expect("spawn 1");
        let id2 = pool.spawn(SpawnConfig::new("receiver")).expect("spawn 2");

        let msg = AgentMessage::new(&id1, &id2, MessageType::Status, "hello from agent 1");
        pool.send_message(msg).expect("send");

        let receiver = pool.get(&id2).unwrap();
        assert_eq!(receiver.inbox.len(), 1);
        assert_eq!(receiver.inbox[0].content, "hello from agent 1");
    }

    #[test]
    fn pool_spawn_decomposed() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 5, max_total: 10, ..Default::default() });
        let base = SpawnConfig::new("");
        let (parent_id, child_ids) = pool
            .spawn_decomposed("add auth", &DecomposeStrategy::ByConcern, &[], &base)
            .expect("decompose");

        assert!(!parent_id.is_empty());
        assert_eq!(child_ids.len(), 3); // implement, test, docs

        let parent = pool.get(&parent_id).unwrap();
        assert_eq!(parent.child_ids.len(), 3);
    }

    #[test]
    fn pool_aggregate_results() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 5, max_total: 10, ..Default::default() });
        let base = SpawnConfig::new("");
        let (parent_id, child_ids) = pool
            .spawn_decomposed("fix bugs", &DecomposeStrategy::ByConcern, &[], &base)
            .expect("decompose");

        // Complete all children
        for cid in &child_ids {
            pool.update_progress(cid, Some("done".into()), &["file.rs".into()], 2, 500).ok();
            pool.complete(cid, "Completed subtask".to_string()).ok();
        }

        let result = pool.aggregate_results(&parent_id).expect("aggregate");
        assert_eq!(result.successful_agents, 3);
        assert_eq!(result.total_tokens_used, 1500);
    }

    #[test]
    fn pool_cleanup() {
        let pool = AgentPool::with_defaults();
        let id = pool.spawn(SpawnConfig::new("cleanup test")).expect("spawn");
        pool.complete(&id, "done".to_string()).expect("complete");

        // Agent should survive cleanup with large age
        let cleaned = pool.cleanup(999_999_999);
        assert_eq!(cleaned, 0);

        // Agent should be cleaned with 0 age
        let cleaned = pool.cleanup(0);
        assert_eq!(cleaned, 1);
        assert!(pool.get(&id).is_none());
    }

    #[test]
    fn pool_stats() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 2, max_total: 10, ..Default::default() });
        let id1 = pool.spawn(SpawnConfig::new("a")).expect("1");
        let _id2 = pool.spawn(SpawnConfig::new("b")).expect("2");
        let _id3 = pool.spawn(SpawnConfig::new("c")).expect("3"); // queued

        pool.complete(&id1, "ok".to_string()).ok();

        let stats = pool.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.running, 2); // id2 + id3 (promoted after id1 completed)
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.max_concurrent, 2);
    }

    #[test]
    fn pool_list_active() {
        let pool = AgentPool::with_defaults();
        let id1 = pool.spawn(SpawnConfig::new("active")).expect("1");
        let id2 = pool.spawn(SpawnConfig::new("will finish")).expect("2");
        pool.complete(&id2, "done".to_string()).ok();

        let active = pool.list_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);
    }

    #[test]
    fn pool_priority_ordering() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 10, ..Default::default() });

        // Fill the single slot
        let id_running = pool.spawn(SpawnConfig::new("running")).expect("running");

        // Queue two agents with different priorities
        let _id_low = pool.spawn(SpawnConfig::new("low pri").with_priority(AgentPriority::Low)).expect("low");
        let id_high = pool.spawn(SpawnConfig::new("high pri").with_priority(AgentPriority::High)).expect("high");

        // Complete running agent — high priority should be promoted first
        pool.complete(&id_running, "done".to_string()).ok();
        let high = pool.get(&id_high).unwrap();
        assert_eq!(high.status, SpawnStatus::Running, "High priority agent should be promoted first");
    }

    #[test]
    fn pool_cannot_pause_queued() {
        let pool = AgentPool::new(PoolConfig { max_concurrent: 1, max_total: 5, ..Default::default() });
        let _id1 = pool.spawn(SpawnConfig::new("running")).expect("1");
        let id2 = pool.spawn(SpawnConfig::new("queued")).expect("2");
        let result = pool.pause(&id2);
        assert!(result.is_err());
    }

    #[test]
    fn pool_cannot_cancel_completed() {
        let pool = AgentPool::with_defaults();
        let id = pool.spawn(SpawnConfig::new("done")).expect("spawn");
        pool.complete(&id, "ok".to_string()).expect("complete");
        let result = pool.cancel(&id);
        assert!(result.is_err());
    }

    // ── SpawnStatus ─────────────────────────────────────────────────────

    #[test]
    fn status_display() {
        assert_eq!(format!("{}", SpawnStatus::Running), "running");
        assert_eq!(format!("{}", SpawnStatus::Queued), "queued");
        assert_eq!(format!("{}", SpawnStatus::Completed), "completed");
    }

    // ── AgentPriority ───────────────────────────────────────────────────

    #[test]
    fn priority_ordering() {
        assert!(AgentPriority::Critical > AgentPriority::High);
        assert!(AgentPriority::High > AgentPriority::Normal);
        assert!(AgentPriority::Normal > AgentPriority::Low);
    }

    // ── MergeStrategy ───────────────────────────────────────────────────

    #[test]
    fn merge_strategies_serialize() {
        let json = serde_json::to_string(&MergeStrategy::BestResult).unwrap();
        assert_eq!(json, r#""best_result""#);

        let json = serde_json::to_string(&MergeStrategy::CherryPick).unwrap();
        assert_eq!(json, r#""cherry_pick""#);
    }

    // ── IsolationMode ───────────────────────────────────────────────────

    #[test]
    fn isolation_mode_default() {
        assert_eq!(IsolationMode::default(), IsolationMode::Worktree);
    }

    // ── PoolStats ───────────────────────────────────────────────────────

    #[test]
    fn pool_stats_display() {
        let stats = PoolStats {
            total: 5,
            running: 2,
            queued: 1,
            completed: 1,
            failed: 1,
            max_concurrent: 5,
            max_total: 20,
            ..Default::default()
        };
        let display = format!("{}", stats);
        assert!(display.contains("2 running"));
        assert!(display.contains("1 queued"));
    }

    // ── Global Pool ─────────────────────────────────────────────────────

    #[test]
    fn global_pool_is_singleton() {
        let p1 = global_pool() as *const AgentPool;
        let p2 = global_pool() as *const AgentPool;
        assert_eq!(p1, p2);
    }

    // ── Serialization roundtrips ────────────────────────────────────────

    #[test]
    fn spawn_config_serde_roundtrip() {
        let cfg = SpawnConfig::new("test task")
            .with_name("test")
            .with_provider("claude")
            .with_tags(vec!["a".into(), "b".into()]);
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: SpawnConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.task, "test task");
        assert_eq!(decoded.name.as_deref(), Some("test"));
        assert_eq!(decoded.tags, vec!["a", "b"]);
    }

    #[test]
    fn spawned_agent_serde_roundtrip() {
        let agent = SpawnedAgent::new(SpawnConfig::new("serialize me"));
        let json = serde_json::to_string(&agent).unwrap();
        let decoded: SpawnedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, agent.id);
        assert_eq!(decoded.task, "serialize me");
    }

    #[test]
    fn aggregated_result_serde() {
        let result = AggregatedResult {
            strategy: MergeStrategy::SequentialMerge,
            total_agents: 2,
            successful_agents: 2,
            failed_agents: 0,
            best_agent_id: Some("sa_1234".into()),
            merged_branch: None,
            summaries: vec![],
            conflicts: vec![],
            total_files_modified: 5,
            total_tokens_used: 10000,
            total_duration_ms: 5000,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("sequential_merge"));
    }
}
