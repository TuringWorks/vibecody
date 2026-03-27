//! Google Agent-to-Agent (A2A) protocol support for VibeCody.
//!
//! Implements the A2A open protocol for agent interoperability, enabling
//! VibeCody agents to discover, negotiate capabilities with, and delegate
//! tasks to other A2A-compatible agents. Includes agent cards, capability
//! negotiation, task lifecycle management, event streaming, and metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Capabilities an A2A agent can advertise.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCapability {
    CodeGeneration,
    CodeReview,
    Testing,
    Debugging,
    Refactoring,
    Documentation,
    Security,
    Deployment,
    DataAnalysis,
    Custom(String),
}

impl AgentCapability {
    pub fn as_str(&self) -> &str {
        match self {
            Self::CodeGeneration => "code_generation",
            Self::CodeReview => "code_review",
            Self::Testing => "testing",
            Self::Debugging => "debugging",
            Self::Refactoring => "refactoring",
            Self::Documentation => "documentation",
            Self::Security => "security",
            Self::Deployment => "deployment",
            Self::DataAnalysis => "data_analysis",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Authentication type for agent endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthType {
    None,
    ApiKey,
    OAuth2,
    Bearer,
}

/// Status of an A2A task through its lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Submitted,
    Working,
    InputNeeded(String),
    Completed,
    Failed(String),
    Canceled,
}

/// Event types emitted by the A2A event stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum A2aEventType {
    TaskCreated,
    StatusChanged,
    OutputReady,
    Error,
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

/// Authentication configuration for an agent endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentAuth {
    pub auth_type: AuthType,
    pub token_url: Option<String>,
    pub scopes: Vec<String>,
}

impl AgentAuth {
    pub fn none() -> Self {
        Self {
            auth_type: AuthType::None,
            token_url: None,
            scopes: Vec::new(),
        }
    }

    pub fn api_key() -> Self {
        Self {
            auth_type: AuthType::ApiKey,
            token_url: None,
            scopes: Vec::new(),
        }
    }

    pub fn oauth2(token_url: &str, scopes: Vec<String>) -> Self {
        Self {
            auth_type: AuthType::OAuth2,
            token_url: Some(token_url.to_string()),
            scopes,
        }
    }

    pub fn bearer() -> Self {
        Self {
            auth_type: AuthType::Bearer,
            token_url: None,
            scopes: Vec::new(),
        }
    }
}

/// A skill advertised by an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

/// A2A Agent Card — the public identity and capability advertisement of an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    pub capabilities: Vec<AgentCapability>,
    pub supported_input_types: Vec<String>,
    pub supported_output_types: Vec<String>,
    pub authentication: AgentAuth,
    pub skills: Vec<AgentSkill>,
}

impl AgentCard {
    pub fn new(name: &str, description: &str, url: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            url: url.to_string(),
            version: version.to_string(),
            capabilities: Vec::new(),
            supported_input_types: vec!["text".to_string()],
            supported_output_types: vec!["text".to_string()],
            authentication: AgentAuth::none(),
            skills: Vec::new(),
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<AgentCapability>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn with_auth(mut self, auth: AgentAuth) -> Self {
        self.authentication = auth;
        self
    }

    pub fn with_skills(mut self, skills: Vec<AgentSkill>) -> Self {
        self.skills = skills;
        self
    }

    pub fn add_capability(&mut self, cap: AgentCapability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    pub fn has_capability(&self, cap: &AgentCapability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Validate that the agent card has required fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Agent name cannot be empty".to_string());
        }
        if self.url.is_empty() {
            return Err("Agent URL cannot be empty".to_string());
        }
        if self.version.is_empty() {
            return Err("Agent version cannot be empty".to_string());
        }
        Ok(())
    }

    /// Serialize the agent card to JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Serialization error: {e}"))
    }

    /// Deserialize an agent card from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialization error: {e}"))
    }
}

/// Input payload for an A2A task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInput {
    pub content_type: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

impl TaskInput {
    pub fn text(content: &str) -> Self {
        Self {
            content_type: "text".to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn code(content: &str, language: &str) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), language.to_string());
        Self {
            content_type: "code".to_string(),
            content: content.to_string(),
            metadata,
        }
    }

    pub fn json(content: &str) -> Self {
        Self {
            content_type: "json".to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn file(path: &str) -> Self {
        Self {
            content_type: "file".to_string(),
            content: path.to_string(),
            metadata: HashMap::new(),
        }
    }
}

/// Output payload from an A2A task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskOutput {
    pub content_type: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

impl TaskOutput {
    pub fn text(content: &str) -> Self {
        Self {
            content_type: "text".to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn code(content: &str, language: &str) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), language.to_string());
        Self {
            content_type: "code".to_string(),
            content: content.to_string(),
            metadata,
        }
    }
}

/// An A2A task representing a unit of work delegated to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aTask {
    pub id: String,
    pub agent_url: String,
    pub input: TaskInput,
    pub status: TaskStatus,
    pub output: Option<TaskOutput>,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: HashMap<String, String>,
}

/// Handler configuration for a registered server handler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandlerConfig {
    pub capability: AgentCapability,
    pub handler_name: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

/// An event in the A2A event stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aEvent {
    pub event_type: A2aEventType,
    pub task_id: String,
    pub timestamp: u64,
    pub data: String,
}

/// Metrics tracking for A2A operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aMetrics {
    pub tasks_created: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_completion_secs: f64,
    pub agents_discovered: u64,
}

impl Default for A2aMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl A2aMetrics {
    pub fn new() -> Self {
        Self {
            tasks_created: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            avg_completion_secs: 0.0,
            agents_discovered: 0,
        }
    }

    pub fn record_created(&mut self) {
        self.tasks_created += 1;
    }

    pub fn record_completed(&mut self, duration_secs: f64) {
        self.tasks_completed += 1;
        // Running average
        let total = self.avg_completion_secs * (self.tasks_completed - 1) as f64 + duration_secs;
        self.avg_completion_secs = total / self.tasks_completed as f64;
    }

    pub fn record_failed(&mut self) {
        self.tasks_failed += 1;
    }

    pub fn record_agent_discovered(&mut self) {
        self.agents_discovered += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 {
            return 0.0;
        }
        self.tasks_completed as f64 / total as f64
    }
}

// ---------------------------------------------------------------------------
// A2aRegistry — agent discovery
// ---------------------------------------------------------------------------

/// Registry for discovering A2A-compatible agents.
pub struct A2aRegistry {
    pub agents: HashMap<String, AgentCard>,
}

impl Default for A2aRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl A2aRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register(&mut self, card: AgentCard) -> Result<(), String> {
        card.validate()?;
        self.agents.insert(card.url.clone(), card);
        Ok(())
    }

    pub fn unregister(&mut self, url: &str) -> bool {
        self.agents.remove(url).is_some()
    }

    pub fn get(&self, url: &str) -> Option<&AgentCard> {
        self.agents.get(url)
    }

    pub fn list(&self) -> Vec<&AgentCard> {
        self.agents.values().collect()
    }

    pub fn find_by_capability(&self, cap: &AgentCapability) -> Vec<&AgentCard> {
        self.agents
            .values()
            .filter(|card| card.has_capability(cap))
            .collect()
    }

    pub fn find_by_name(&self, name: &str) -> Option<&AgentCard> {
        self.agents.values().find(|c| c.name == name)
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }
}

// ---------------------------------------------------------------------------
// CapabilityNegotiator
// ---------------------------------------------------------------------------

/// Negotiates capability matching between agents.
pub struct CapabilityNegotiator;

impl Default for CapabilityNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityNegotiator {
    pub fn new() -> Self {
        Self
    }

    /// Score how well `offered` capabilities match `required` ones.
    /// Returns 0.0 (no match) to 1.0 (full match).
    pub fn match_score(required: &[AgentCapability], offered: &[AgentCapability]) -> f64 {
        if required.is_empty() {
            return 1.0;
        }
        let matched = required.iter().filter(|r| offered.contains(r)).count();
        matched as f64 / required.len() as f64
    }

    /// Find the best agent in a registry for the required capabilities.
    pub fn find_best_agent<'a>(
        &self,
        registry: &'a A2aRegistry,
        required: &[AgentCapability],
    ) -> Option<&'a AgentCard> {
        registry
            .agents
            .values()
            .max_by(|a, b| {
                let score_a = Self::match_score(required, &a.capabilities);
                let score_b = Self::match_score(required, &b.capabilities);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .filter(|card| Self::match_score(required, &card.capabilities) > 0.0)
    }

    /// Find all agents meeting a minimum match threshold.
    pub fn find_agents_above_threshold<'a>(
        &self,
        registry: &'a A2aRegistry,
        required: &[AgentCapability],
        threshold: f64,
    ) -> Vec<(&'a AgentCard, f64)> {
        registry
            .agents
            .values()
            .filter_map(|card| {
                let score = Self::match_score(required, &card.capabilities);
                if score >= threshold {
                    Some((card, score))
                } else {
                    None
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// TaskLifecycleManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of A2A tasks.
pub struct TaskLifecycleManager {
    tasks: HashMap<String, A2aTask>,
    next_id: u64,
    timestamp_counter: u64,
}

impl Default for TaskLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskLifecycleManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
            timestamp_counter: 0,
        }
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn create_task(&mut self, agent_url: &str, input: TaskInput) -> String {
        let id = format!("task-{}", self.next_id);
        self.next_id += 1;
        let ts = self.next_timestamp();
        let task = A2aTask {
            id: id.clone(),
            agent_url: agent_url.to_string(),
            input,
            status: TaskStatus::Submitted,
            output: None,
            created_at: ts,
            updated_at: ts,
            metadata: HashMap::new(),
        };
        self.tasks.insert(id.clone(), task);
        id
    }

    pub fn update_status(&mut self, task_id: &str, status: TaskStatus) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {task_id}"))?;
        match &task.status {
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Canceled => {
                return Err(format!(
                    "Cannot update task in terminal state: {:?}",
                    task.status
                ));
            }
            _ => {}
        }
        task.status = status;
        task.updated_at = ts;
        Ok(())
    }

    pub fn complete_task(&mut self, task_id: &str, output: TaskOutput) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {task_id}"))?;
        match &task.status {
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Canceled => {
                return Err(format!(
                    "Cannot complete task in terminal state: {:?}",
                    task.status
                ));
            }
            _ => {}
        }
        task.status = TaskStatus::Completed;
        task.output = Some(output);
        task.updated_at = ts;
        Ok(())
    }

    pub fn fail_task(&mut self, task_id: &str, reason: &str) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {task_id}"))?;
        match &task.status {
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Canceled => {
                return Err(format!(
                    "Cannot fail task in terminal state: {:?}",
                    task.status
                ));
            }
            _ => {}
        }
        task.status = TaskStatus::Failed(reason.to_string());
        task.updated_at = ts;
        Ok(())
    }

    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {task_id}"))?;
        match &task.status {
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Canceled => {
                return Err(format!(
                    "Cannot cancel task in terminal state: {:?}",
                    task.status
                ));
            }
            _ => {}
        }
        task.status = TaskStatus::Canceled;
        task.updated_at = ts;
        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> Option<&A2aTask> {
        self.tasks.get(task_id)
    }

    pub fn list_tasks(&self) -> Vec<&A2aTask> {
        self.tasks.values().collect()
    }

    pub fn list_tasks_by_status(&self, status: &TaskStatus) -> Vec<&A2aTask> {
        self.tasks
            .values()
            .filter(|t| std::mem::discriminant(&t.status) == std::mem::discriminant(status))
            .collect()
    }

    pub fn cleanup_completed(&mut self) -> usize {
        let before = self.tasks.len();
        self.tasks.retain(|_, t| {
            !matches!(
                t.status,
                TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Canceled
            )
        });
        before - self.tasks.len()
    }

    pub fn count(&self) -> usize {
        self.tasks.len()
    }
}

// ---------------------------------------------------------------------------
// A2aEventStream
// ---------------------------------------------------------------------------

/// Records and replays events from A2A task operations.
pub struct A2aEventStream {
    events: Vec<A2aEvent>,
    timestamp_counter: u64,
}

impl Default for A2aEventStream {
    fn default() -> Self {
        Self::new()
    }
}

impl A2aEventStream {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            timestamp_counter: 0,
        }
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn emit(&mut self, event_type: A2aEventType, task_id: &str, data: &str) {
        let ts = self.next_timestamp();
        self.events.push(A2aEvent {
            event_type,
            task_id: task_id.to_string(),
            timestamp: ts,
            data: data.to_string(),
        });
    }

    pub fn events_for_task(&self, task_id: &str) -> Vec<&A2aEvent> {
        self.events.iter().filter(|e| e.task_id == task_id).collect()
    }

    pub fn events_by_type(&self, event_type: &A2aEventType) -> Vec<&A2aEvent> {
        self.events
            .iter()
            .filter(|e| &e.event_type == event_type)
            .collect()
    }

    pub fn all_events(&self) -> &[A2aEvent] {
        &self.events
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn last_event(&self) -> Option<&A2aEvent> {
        self.events.last()
    }
}

// ---------------------------------------------------------------------------
// A2aServer
// ---------------------------------------------------------------------------

/// An A2A server that registers handlers and manages incoming tasks.
pub struct A2aServer {
    pub hostname: String,
    pub port: u16,
    pub agent_card: AgentCard,
    pub registered_handlers: HashMap<String, HandlerConfig>,
    pub active_tasks: Vec<A2aTask>,
    pub max_concurrent: u32,
    event_stream: A2aEventStream,
    metrics: A2aMetrics,
    next_task_id: u64,
    timestamp_counter: u64,
}

impl A2aServer {
    pub fn new(hostname: &str, port: u16, agent_card: AgentCard) -> Self {
        Self {
            hostname: hostname.to_string(),
            port,
            agent_card,
            registered_handlers: HashMap::new(),
            active_tasks: Vec::new(),
            max_concurrent: 10,
            event_stream: A2aEventStream::new(),
            metrics: A2aMetrics::new(),
            next_task_id: 1,
            timestamp_counter: 0,
        }
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn register_handler(&mut self, config: HandlerConfig) {
        self.registered_handlers
            .insert(config.handler_name.clone(), config);
    }

    pub fn unregister_handler(&mut self, handler_name: &str) -> bool {
        self.registered_handlers.remove(handler_name).is_some()
    }

    pub fn has_handler(&self, handler_name: &str) -> bool {
        self.registered_handlers.contains_key(handler_name)
    }

    pub fn submit_task(&mut self, input: TaskInput) -> Result<String, String> {
        if self.active_tasks.len() >= self.max_concurrent as usize {
            return Err("Max concurrent tasks reached".to_string());
        }
        let id = format!("srv-task-{}", self.next_task_id);
        self.next_task_id += 1;
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = A2aTask {
            id: id.clone(),
            agent_url: format!("{}:{}", self.hostname, self.port),
            input,
            status: TaskStatus::Submitted,
            output: None,
            created_at: ts,
            updated_at: ts,
            metadata: HashMap::new(),
        };
        self.active_tasks.push(task);
        self.metrics.record_created();
        self.event_stream
            .emit(A2aEventType::TaskCreated, &id, "Task submitted");
        Ok(id)
    }

    pub fn complete_task(&mut self, task_id: &str, output: TaskOutput) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        {
            let task = self
                .active_tasks
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task not found: {task_id}"))?;
            task.status = TaskStatus::Completed;
            task.output = Some(output);
            task.updated_at = ts;
        }
        self.metrics.record_completed(1.0);
        self.event_stream
            .emit(A2aEventType::OutputReady, task_id, "Task completed");
        Ok(())
    }

    pub fn fail_task(&mut self, task_id: &str, reason: &str) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        {
            let task = self
                .active_tasks
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task not found: {task_id}"))?;
            task.status = TaskStatus::Failed(reason.to_string());
            task.updated_at = ts;
        }
        self.metrics.record_failed();
        self.event_stream
            .emit(A2aEventType::Error, task_id, reason);
        Ok(())
    }

    pub fn get_metrics(&self) -> &A2aMetrics {
        &self.metrics
    }

    pub fn get_events(&self) -> &A2aEventStream {
        &self.event_stream
    }

    pub fn handler_count(&self) -> usize {
        self.registered_handlers.len()
    }

    pub fn active_task_count(&self) -> usize {
        self.active_tasks.len()
    }

    pub fn endpoint_url(&self) -> String {
        format!("http://{}:{}", self.hostname, self.port)
    }
}

// ---------------------------------------------------------------------------
// A2aClient
// ---------------------------------------------------------------------------

/// An A2A client for discovering agents and submitting tasks.
pub struct A2aClient {
    pub known_agents: Vec<AgentCard>,
    pub task_history: Vec<A2aTask>,
    pub timeout_secs: u64,
    pub retry_count: u32,
    metrics: A2aMetrics,
    next_task_id: u64,
    timestamp_counter: u64,
}

impl A2aClient {
    pub fn new(timeout_secs: u64, retry_count: u32) -> Self {
        Self {
            known_agents: Vec::new(),
            task_history: Vec::new(),
            timeout_secs,
            retry_count,
            metrics: A2aMetrics::new(),
            next_task_id: 1,
            timestamp_counter: 0,
        }
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn discover_agent(&mut self, card: AgentCard) -> Result<(), String> {
        card.validate()?;
        if self.known_agents.iter().any(|a| a.url == card.url) {
            return Err(format!("Agent already known: {}", card.url));
        }
        self.known_agents.push(card);
        self.metrics.record_agent_discovered();
        Ok(())
    }

    pub fn forget_agent(&mut self, url: &str) -> bool {
        let before = self.known_agents.len();
        self.known_agents.retain(|a| a.url != url);
        self.known_agents.len() < before
    }

    pub fn find_agent(&self, url: &str) -> Option<&AgentCard> {
        self.known_agents.iter().find(|a| a.url == url)
    }

    pub fn find_agents_by_capability(&self, cap: &AgentCapability) -> Vec<&AgentCard> {
        self.known_agents
            .iter()
            .filter(|a| a.has_capability(cap))
            .collect()
    }

    pub fn submit_task(&mut self, agent_url: &str, input: TaskInput) -> Result<String, String> {
        if !self.known_agents.iter().any(|a| a.url == agent_url) {
            return Err(format!("Unknown agent: {agent_url}"));
        }
        let id = format!("client-task-{}", self.next_task_id);
        self.next_task_id += 1;
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        let task = A2aTask {
            id: id.clone(),
            agent_url: agent_url.to_string(),
            input,
            status: TaskStatus::Submitted,
            output: None,
            created_at: ts,
            updated_at: ts,
            metadata: HashMap::new(),
        };
        self.task_history.push(task);
        self.metrics.record_created();
        Ok(id)
    }

    pub fn mark_task_completed(
        &mut self,
        task_id: &str,
        output: TaskOutput,
    ) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        {
            let task = self
                .task_history
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task not found: {task_id}"))?;
            task.status = TaskStatus::Completed;
            task.output = Some(output);
            task.updated_at = ts;
        }
        self.metrics.record_completed(1.0);
        Ok(())
    }

    pub fn mark_task_failed(&mut self, task_id: &str, reason: &str) -> Result<(), String> {
        self.timestamp_counter += 1;
        let ts = self.timestamp_counter;
        {
            let task = self
                .task_history
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task not found: {task_id}"))?;
            task.status = TaskStatus::Failed(reason.to_string());
            task.updated_at = ts;
        }
        self.metrics.record_failed();
        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> Option<&A2aTask> {
        self.task_history.iter().find(|t| t.id == task_id)
    }

    pub fn get_metrics(&self) -> &A2aMetrics {
        &self.metrics
    }

    pub fn agent_count(&self) -> usize {
        self.known_agents.len()
    }

    pub fn task_count(&self) -> usize {
        self.task_history.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers --

    fn make_agent_card(name: &str, url: &str) -> AgentCard {
        AgentCard::new(name, &format!("{name} agent"), url, "1.0.0")
    }

    fn make_capable_card(name: &str, url: &str, caps: Vec<AgentCapability>) -> AgentCard {
        AgentCard::new(name, &format!("{name} agent"), url, "1.0.0").with_capabilities(caps)
    }

    fn make_skill(name: &str) -> AgentSkill {
        AgentSkill {
            name: name.to_string(),
            description: format!("{name} skill"),
            input_types: vec!["text".to_string()],
            output_types: vec!["text".to_string()],
        }
    }

    fn make_handler(name: &str, cap: AgentCapability) -> HandlerConfig {
        HandlerConfig {
            capability: cap,
            handler_name: name.to_string(),
            timeout_secs: 30,
            max_retries: 3,
        }
    }

    fn make_registry_with_agents() -> A2aRegistry {
        let mut reg = A2aRegistry::new();
        reg.register(make_capable_card(
            "coder",
            "http://coder:8080",
            vec![AgentCapability::CodeGeneration, AgentCapability::Refactoring],
        ))
        .unwrap();
        reg.register(make_capable_card(
            "reviewer",
            "http://reviewer:8080",
            vec![AgentCapability::CodeReview, AgentCapability::Security],
        ))
        .unwrap();
        reg.register(make_capable_card(
            "tester",
            "http://tester:8080",
            vec![AgentCapability::Testing, AgentCapability::Debugging],
        ))
        .unwrap();
        reg
    }

    // -- AgentCard tests --

    #[test]
    fn test_agent_card_creation() {
        let card = make_agent_card("test", "http://localhost:8080");
        assert_eq!(card.name, "test");
        assert_eq!(card.url, "http://localhost:8080");
        assert_eq!(card.version, "1.0.0");
        assert!(card.capabilities.is_empty());
    }

    #[test]
    fn test_agent_card_with_capabilities() {
        let card = make_capable_card(
            "test",
            "http://localhost:8080",
            vec![AgentCapability::CodeGeneration, AgentCapability::Testing],
        );
        assert_eq!(card.capabilities.len(), 2);
        assert!(card.has_capability(&AgentCapability::CodeGeneration));
        assert!(!card.has_capability(&AgentCapability::Security));
    }

    #[test]
    fn test_agent_card_add_capability_dedup() {
        let mut card = make_agent_card("test", "http://localhost:8080");
        card.add_capability(AgentCapability::Testing);
        card.add_capability(AgentCapability::Testing);
        assert_eq!(card.capabilities.len(), 1);
    }

    #[test]
    fn test_agent_card_with_auth() {
        let card = make_agent_card("test", "http://localhost:8080")
            .with_auth(AgentAuth::oauth2("https://auth.example.com/token", vec!["read".to_string()]));
        assert_eq!(card.authentication.auth_type, AuthType::OAuth2);
        assert_eq!(
            card.authentication.token_url,
            Some("https://auth.example.com/token".to_string())
        );
        assert_eq!(card.authentication.scopes, vec!["read"]);
    }

    #[test]
    fn test_agent_card_with_skills() {
        let card =
            make_agent_card("test", "http://localhost:8080").with_skills(vec![make_skill("rust")]);
        assert_eq!(card.skills.len(), 1);
        assert_eq!(card.skills[0].name, "rust");
    }

    #[test]
    fn test_agent_card_validate_ok() {
        let card = make_agent_card("test", "http://localhost:8080");
        assert!(card.validate().is_ok());
    }

    #[test]
    fn test_agent_card_validate_empty_name() {
        let card = AgentCard::new("", "desc", "http://localhost", "1.0");
        assert!(card.validate().is_err());
    }

    #[test]
    fn test_agent_card_validate_empty_url() {
        let card = AgentCard::new("name", "desc", "", "1.0");
        assert!(card.validate().is_err());
    }

    #[test]
    fn test_agent_card_validate_empty_version() {
        let card = AgentCard::new("name", "desc", "http://localhost", "");
        assert!(card.validate().is_err());
    }

    #[test]
    fn test_agent_card_serialization_roundtrip() {
        let card = make_capable_card(
            "test",
            "http://localhost:8080",
            vec![AgentCapability::CodeGeneration],
        )
        .with_auth(AgentAuth::api_key())
        .with_skills(vec![make_skill("python")]);

        let json = card.to_json().expect("serialize");
        let parsed = AgentCard::from_json(&json).expect("deserialize");
        assert_eq!(card, parsed);
    }

    #[test]
    fn test_agent_card_json_contains_fields() {
        let card = make_agent_card("myagent", "http://example.com");
        let json = card.to_json().unwrap();
        assert!(json.contains("myagent"));
        assert!(json.contains("http://example.com"));
    }

    #[test]
    fn test_agent_card_from_invalid_json() {
        let result = AgentCard::from_json("not json");
        assert!(result.is_err());
    }

    // -- AgentCapability tests --

    #[test]
    fn test_capability_as_str() {
        assert_eq!(AgentCapability::CodeGeneration.as_str(), "code_generation");
        assert_eq!(AgentCapability::Security.as_str(), "security");
        assert_eq!(
            AgentCapability::Custom("my_cap".to_string()).as_str(),
            "my_cap"
        );
    }

    #[test]
    fn test_capability_equality() {
        assert_eq!(AgentCapability::Testing, AgentCapability::Testing);
        assert_ne!(AgentCapability::Testing, AgentCapability::Debugging);
        assert_eq!(
            AgentCapability::Custom("x".into()),
            AgentCapability::Custom("x".into())
        );
        assert_ne!(
            AgentCapability::Custom("x".into()),
            AgentCapability::Custom("y".into())
        );
    }

    // -- AgentAuth tests --

    #[test]
    fn test_auth_none() {
        let auth = AgentAuth::none();
        assert_eq!(auth.auth_type, AuthType::None);
        assert!(auth.token_url.is_none());
    }

    #[test]
    fn test_auth_bearer() {
        let auth = AgentAuth::bearer();
        assert_eq!(auth.auth_type, AuthType::Bearer);
    }

    // -- TaskInput / TaskOutput tests --

    #[test]
    fn test_task_input_text() {
        let input = TaskInput::text("hello");
        assert_eq!(input.content_type, "text");
        assert_eq!(input.content, "hello");
    }

    #[test]
    fn test_task_input_code() {
        let input = TaskInput::code("fn main() {}", "rust");
        assert_eq!(input.content_type, "code");
        assert_eq!(input.metadata.get("language").unwrap(), "rust");
    }

    #[test]
    fn test_task_input_json() {
        let input = TaskInput::json(r#"{"key":"val"}"#);
        assert_eq!(input.content_type, "json");
    }

    #[test]
    fn test_task_input_file() {
        let input = TaskInput::file("/tmp/data.txt");
        assert_eq!(input.content_type, "file");
        assert_eq!(input.content, "/tmp/data.txt");
    }

    #[test]
    fn test_task_output_code() {
        let out = TaskOutput::code("println!()", "rust");
        assert_eq!(out.content_type, "code");
        assert_eq!(out.metadata.get("language").unwrap(), "rust");
    }

    // -- A2aRegistry tests --

    #[test]
    fn test_registry_register_and_get() {
        let mut reg = A2aRegistry::new();
        let card = make_agent_card("a", "http://a:8080");
        reg.register(card).unwrap();
        assert_eq!(reg.count(), 1);
        assert!(reg.get("http://a:8080").is_some());
    }

    #[test]
    fn test_registry_register_invalid() {
        let mut reg = A2aRegistry::new();
        let card = AgentCard::new("", "d", "http://x", "1");
        assert!(reg.register(card).is_err());
    }

    #[test]
    fn test_registry_unregister() {
        let mut reg = A2aRegistry::new();
        reg.register(make_agent_card("a", "http://a:8080")).unwrap();
        assert!(reg.unregister("http://a:8080"));
        assert!(!reg.unregister("http://a:8080"));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_find_by_capability() {
        let reg = make_registry_with_agents();
        let results = reg.find_by_capability(&AgentCapability::CodeGeneration);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "coder");
    }

    #[test]
    fn test_registry_find_by_name() {
        let reg = make_registry_with_agents();
        assert!(reg.find_by_name("reviewer").is_some());
        assert!(reg.find_by_name("unknown").is_none());
    }

    #[test]
    fn test_registry_list() {
        let reg = make_registry_with_agents();
        assert_eq!(reg.list().len(), 3);
    }

    // -- CapabilityNegotiator tests --

    #[test]
    fn test_match_score_full() {
        let score = CapabilityNegotiator::match_score(
            &[AgentCapability::Testing],
            &[AgentCapability::Testing, AgentCapability::Debugging],
        );
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_match_score_partial() {
        let score = CapabilityNegotiator::match_score(
            &[AgentCapability::Testing, AgentCapability::Security],
            &[AgentCapability::Testing],
        );
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_match_score_none() {
        let score = CapabilityNegotiator::match_score(
            &[AgentCapability::Deployment],
            &[AgentCapability::Testing],
        );
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_match_score_empty_required() {
        let score =
            CapabilityNegotiator::match_score(&[], &[AgentCapability::Testing]);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_best_agent() {
        let reg = make_registry_with_agents();
        let negotiator = CapabilityNegotiator::new();
        let best = negotiator
            .find_best_agent(&reg, &[AgentCapability::CodeGeneration])
            .unwrap();
        assert_eq!(best.name, "coder");
    }

    #[test]
    fn test_find_best_agent_no_match() {
        let reg = make_registry_with_agents();
        let negotiator = CapabilityNegotiator::new();
        let best = negotiator.find_best_agent(&reg, &[AgentCapability::Deployment]);
        assert!(best.is_none());
    }

    #[test]
    fn test_find_agents_above_threshold() {
        let reg = make_registry_with_agents();
        let negotiator = CapabilityNegotiator::new();
        let results = negotiator.find_agents_above_threshold(
            &reg,
            &[AgentCapability::CodeReview, AgentCapability::Security],
            0.5,
        );
        assert!(results.iter().any(|(c, _)| c.name == "reviewer"));
    }

    // -- TaskLifecycleManager tests --

    #[test]
    fn test_create_task() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://agent:8080", TaskInput::text("do something"));
        assert_eq!(id, "task-1");
        assert_eq!(mgr.count(), 1);
        let task = mgr.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Submitted);
    }

    #[test]
    fn test_task_lifecycle_submit_to_complete() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.update_status(&id, TaskStatus::Working).unwrap();
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Working);
        mgr.complete_task(&id, TaskOutput::text("done")).unwrap();
        let task = mgr.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output.is_some());
    }

    #[test]
    fn test_task_lifecycle_submit_to_fail() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.update_status(&id, TaskStatus::Working).unwrap();
        mgr.fail_task(&id, "timeout").unwrap();
        let task = mgr.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Failed("timeout".to_string()));
    }

    #[test]
    fn test_task_lifecycle_submit_to_cancel() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.cancel_task(&id).unwrap();
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Canceled);
    }

    #[test]
    fn test_cannot_update_completed_task() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.complete_task(&id, TaskOutput::text("ok")).unwrap();
        assert!(mgr.update_status(&id, TaskStatus::Working).is_err());
        assert!(mgr.fail_task(&id, "nope").is_err());
        assert!(mgr.cancel_task(&id).is_err());
    }

    #[test]
    fn test_cannot_update_failed_task() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.fail_task(&id, "boom").unwrap();
        assert!(mgr.complete_task(&id, TaskOutput::text("ok")).is_err());
    }

    #[test]
    fn test_cannot_update_canceled_task() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.cancel_task(&id).unwrap();
        assert!(mgr.update_status(&id, TaskStatus::Working).is_err());
    }

    #[test]
    fn test_update_nonexistent_task() {
        let mut mgr = TaskLifecycleManager::new();
        assert!(mgr.update_status("nope", TaskStatus::Working).is_err());
        assert!(mgr.fail_task("nope", "x").is_err());
        assert!(mgr.cancel_task("nope").is_err());
        assert!(mgr.complete_task("nope", TaskOutput::text("x")).is_err());
    }

    #[test]
    fn test_input_needed_status() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("work"));
        mgr.update_status(&id, TaskStatus::InputNeeded("need file path".into()))
            .unwrap();
        match &mgr.get_task(&id).unwrap().status {
            TaskStatus::InputNeeded(msg) => assert_eq!(msg, "need file path"),
            _ => panic!("expected InputNeeded"),
        }
    }

    #[test]
    fn test_list_tasks_by_status() {
        let mut mgr = TaskLifecycleManager::new();
        let id1 = mgr.create_task("http://a:8080", TaskInput::text("a"));
        let _id2 = mgr.create_task("http://a:8080", TaskInput::text("b"));
        mgr.update_status(&id1, TaskStatus::Working).unwrap();
        let working = mgr.list_tasks_by_status(&TaskStatus::Working);
        assert_eq!(working.len(), 1);
        let submitted = mgr.list_tasks_by_status(&TaskStatus::Submitted);
        assert_eq!(submitted.len(), 1);
    }

    #[test]
    fn test_cleanup_completed() {
        let mut mgr = TaskLifecycleManager::new();
        let id1 = mgr.create_task("http://a:8080", TaskInput::text("a"));
        let _id2 = mgr.create_task("http://a:8080", TaskInput::text("b"));
        mgr.complete_task(&id1, TaskOutput::text("done")).unwrap();
        let removed = mgr.cleanup_completed();
        assert_eq!(removed, 1);
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_list_tasks() {
        let mut mgr = TaskLifecycleManager::new();
        mgr.create_task("http://a:8080", TaskInput::text("a"));
        mgr.create_task("http://a:8080", TaskInput::text("b"));
        assert_eq!(mgr.list_tasks().len(), 2);
    }

    #[test]
    fn test_task_timestamps_increase() {
        let mut mgr = TaskLifecycleManager::new();
        let id = mgr.create_task("http://a:8080", TaskInput::text("a"));
        let created = mgr.get_task(&id).unwrap().created_at;
        mgr.update_status(&id, TaskStatus::Working).unwrap();
        let updated = mgr.get_task(&id).unwrap().updated_at;
        assert!(updated > created);
    }

    // -- A2aEventStream tests --

    #[test]
    fn test_event_stream_emit_and_count() {
        let mut stream = A2aEventStream::new();
        stream.emit(A2aEventType::TaskCreated, "t1", "created");
        stream.emit(A2aEventType::StatusChanged, "t1", "working");
        assert_eq!(stream.count(), 2);
    }

    #[test]
    fn test_event_stream_filter_by_task() {
        let mut stream = A2aEventStream::new();
        stream.emit(A2aEventType::TaskCreated, "t1", "a");
        stream.emit(A2aEventType::TaskCreated, "t2", "b");
        stream.emit(A2aEventType::StatusChanged, "t1", "c");
        assert_eq!(stream.events_for_task("t1").len(), 2);
        assert_eq!(stream.events_for_task("t2").len(), 1);
    }

    #[test]
    fn test_event_stream_filter_by_type() {
        let mut stream = A2aEventStream::new();
        stream.emit(A2aEventType::TaskCreated, "t1", "a");
        stream.emit(A2aEventType::Error, "t1", "b");
        stream.emit(A2aEventType::Error, "t2", "c");
        assert_eq!(stream.events_by_type(&A2aEventType::Error).len(), 2);
    }

    #[test]
    fn test_event_stream_clear() {
        let mut stream = A2aEventStream::new();
        stream.emit(A2aEventType::TaskCreated, "t1", "a");
        stream.clear();
        assert_eq!(stream.count(), 0);
    }

    #[test]
    fn test_event_stream_last_event() {
        let mut stream = A2aEventStream::new();
        assert!(stream.last_event().is_none());
        stream.emit(A2aEventType::TaskCreated, "t1", "first");
        stream.emit(A2aEventType::OutputReady, "t1", "second");
        assert_eq!(stream.last_event().unwrap().data, "second");
    }

    #[test]
    fn test_event_timestamps_increase() {
        let mut stream = A2aEventStream::new();
        stream.emit(A2aEventType::TaskCreated, "t1", "a");
        stream.emit(A2aEventType::StatusChanged, "t1", "b");
        let events = stream.all_events();
        assert!(events[1].timestamp > events[0].timestamp);
    }

    // -- A2aServer tests --

    #[test]
    fn test_server_creation() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let server = A2aServer::new("localhost", 9000, card);
        assert_eq!(server.hostname, "localhost");
        assert_eq!(server.port, 9000);
        assert_eq!(server.endpoint_url(), "http://localhost:9000");
    }

    #[test]
    fn test_server_register_handler() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        server.register_handler(make_handler("code_handler", AgentCapability::CodeGeneration));
        assert!(server.has_handler("code_handler"));
        assert_eq!(server.handler_count(), 1);
    }

    #[test]
    fn test_server_unregister_handler() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        server.register_handler(make_handler("h1", AgentCapability::Testing));
        assert!(server.unregister_handler("h1"));
        assert!(!server.unregister_handler("h1"));
        assert_eq!(server.handler_count(), 0);
    }

    #[test]
    fn test_server_submit_and_complete_task() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        let id = server.submit_task(TaskInput::text("build it")).unwrap();
        assert_eq!(server.active_task_count(), 1);
        server.complete_task(&id, TaskOutput::text("built")).unwrap();
        let metrics = server.get_metrics();
        assert_eq!(metrics.tasks_created, 1);
        assert_eq!(metrics.tasks_completed, 1);
    }

    #[test]
    fn test_server_submit_and_fail_task() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        let id = server.submit_task(TaskInput::text("fail")).unwrap();
        server.fail_task(&id, "crash").unwrap();
        assert_eq!(server.get_metrics().tasks_failed, 1);
    }

    #[test]
    fn test_server_max_concurrent() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        server.max_concurrent = 2;
        server.submit_task(TaskInput::text("a")).unwrap();
        server.submit_task(TaskInput::text("b")).unwrap();
        let result = server.submit_task(TaskInput::text("c"));
        assert!(result.is_err());
    }

    #[test]
    fn test_server_events_recorded() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        let id = server.submit_task(TaskInput::text("x")).unwrap();
        server.complete_task(&id, TaskOutput::text("y")).unwrap();
        assert_eq!(server.get_events().count(), 2);
    }

    #[test]
    fn test_server_fail_nonexistent_task() {
        let card = make_agent_card("srv", "http://localhost:9000");
        let mut server = A2aServer::new("localhost", 9000, card);
        assert!(server.fail_task("nope", "x").is_err());
    }

    // -- A2aClient tests --

    #[test]
    fn test_client_discover_agent() {
        let mut client = A2aClient::new(30, 3);
        let card = make_agent_card("remote", "http://remote:8080");
        client.discover_agent(card).unwrap();
        assert_eq!(client.agent_count(), 1);
        assert!(client.find_agent("http://remote:8080").is_some());
    }

    #[test]
    fn test_client_discover_duplicate() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_agent_card("a", "http://a:8080"))
            .unwrap();
        assert!(client
            .discover_agent(make_agent_card("a2", "http://a:8080"))
            .is_err());
    }

    #[test]
    fn test_client_discover_invalid() {
        let mut client = A2aClient::new(30, 3);
        let card = AgentCard::new("", "d", "http://x", "1");
        assert!(client.discover_agent(card).is_err());
    }

    #[test]
    fn test_client_forget_agent() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_agent_card("a", "http://a:8080"))
            .unwrap();
        assert!(client.forget_agent("http://a:8080"));
        assert!(!client.forget_agent("http://a:8080"));
        assert_eq!(client.agent_count(), 0);
    }

    #[test]
    fn test_client_find_by_capability() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_capable_card(
                "sec",
                "http://sec:8080",
                vec![AgentCapability::Security],
            ))
            .unwrap();
        client
            .discover_agent(make_capable_card(
                "test",
                "http://test:8080",
                vec![AgentCapability::Testing],
            ))
            .unwrap();
        let sec_agents = client.find_agents_by_capability(&AgentCapability::Security);
        assert_eq!(sec_agents.len(), 1);
        assert_eq!(sec_agents[0].name, "sec");
    }

    #[test]
    fn test_client_submit_task() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_agent_card("a", "http://a:8080"))
            .unwrap();
        let id = client
            .submit_task("http://a:8080", TaskInput::text("hello"))
            .unwrap();
        assert_eq!(client.task_count(), 1);
        let task = client.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Submitted);
    }

    #[test]
    fn test_client_submit_to_unknown_agent() {
        let mut client = A2aClient::new(30, 3);
        assert!(client
            .submit_task("http://unknown:8080", TaskInput::text("x"))
            .is_err());
    }

    #[test]
    fn test_client_mark_completed() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_agent_card("a", "http://a:8080"))
            .unwrap();
        let id = client
            .submit_task("http://a:8080", TaskInput::text("x"))
            .unwrap();
        client
            .mark_task_completed(&id, TaskOutput::text("result"))
            .unwrap();
        assert_eq!(client.get_task(&id).unwrap().status, TaskStatus::Completed);
        assert_eq!(client.get_metrics().tasks_completed, 1);
    }

    #[test]
    fn test_client_mark_failed() {
        let mut client = A2aClient::new(30, 3);
        client
            .discover_agent(make_agent_card("a", "http://a:8080"))
            .unwrap();
        let id = client
            .submit_task("http://a:8080", TaskInput::text("x"))
            .unwrap();
        client.mark_task_failed(&id, "error").unwrap();
        assert_eq!(client.get_metrics().tasks_failed, 1);
    }

    #[test]
    fn test_client_mark_nonexistent_task() {
        let mut client = A2aClient::new(30, 3);
        assert!(client
            .mark_task_completed("nope", TaskOutput::text("x"))
            .is_err());
        assert!(client.mark_task_failed("nope", "x").is_err());
    }

    // -- A2aMetrics tests --

    #[test]
    fn test_metrics_initial_state() {
        let m = A2aMetrics::new();
        assert_eq!(m.tasks_created, 0);
        assert_eq!(m.tasks_completed, 0);
        assert_eq!(m.tasks_failed, 0);
        assert!((m.avg_completion_secs - 0.0).abs() < f64::EPSILON);
        assert!((m.success_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_success_rate() {
        let mut m = A2aMetrics::new();
        m.record_completed(1.0);
        m.record_completed(2.0);
        m.record_failed();
        let rate = m.success_rate();
        assert!((rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_avg_completion() {
        let mut m = A2aMetrics::new();
        m.record_completed(2.0);
        m.record_completed(4.0);
        assert!((m.avg_completion_secs - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_agent_discovered() {
        let mut m = A2aMetrics::new();
        m.record_agent_discovered();
        m.record_agent_discovered();
        assert_eq!(m.agents_discovered, 2);
    }

    // -- HandlerConfig test --

    #[test]
    fn test_handler_config_fields() {
        let h = make_handler("review", AgentCapability::CodeReview);
        assert_eq!(h.handler_name, "review");
        assert_eq!(h.capability, AgentCapability::CodeReview);
        assert_eq!(h.timeout_secs, 30);
        assert_eq!(h.max_retries, 3);
    }

    // -- Edge cases --

    #[test]
    fn test_empty_registry_find() {
        let reg = A2aRegistry::new();
        assert!(reg.find_by_capability(&AgentCapability::Testing).is_empty());
        assert!(reg.find_by_name("x").is_none());
    }

    #[test]
    fn test_negotiator_empty_registry() {
        let reg = A2aRegistry::new();
        let neg = CapabilityNegotiator::new();
        assert!(neg
            .find_best_agent(&reg, &[AgentCapability::Testing])
            .is_none());
    }

    #[test]
    fn test_cleanup_with_no_completed() {
        let mut mgr = TaskLifecycleManager::new();
        mgr.create_task("http://a:8080", TaskInput::text("a"));
        assert_eq!(mgr.cleanup_completed(), 0);
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_multiple_task_ids_unique() {
        let mut mgr = TaskLifecycleManager::new();
        let id1 = mgr.create_task("http://a:8080", TaskInput::text("a"));
        let id2 = mgr.create_task("http://a:8080", TaskInput::text("b"));
        let id3 = mgr.create_task("http://a:8080", TaskInput::text("c"));
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }
}
