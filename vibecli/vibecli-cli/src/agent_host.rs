//! Multi-agent terminal hosting for VibeCLI.
//!
//! Provides the ability to register, manage, and coordinate multiple external
//! AI coding agents (Claude Code, Gemini CLI, Aider, Cline, Goose, or custom)
//! within a single terminal session. Includes shared context clipboard,
//! output routing/interleaving, capability-based agent selection, and metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The type of external agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentType {
    ClaudeCode,
    GeminiCli,
    Aider,
    Cline,
    Goose,
    Custom(String),
}

impl AgentType {
    /// Human-readable label used for metrics keys.
    pub fn label(&self) -> String {
        match self {
            AgentType::ClaudeCode => "claude_code".to_string(),
            AgentType::GeminiCli => "gemini_cli".to_string(),
            AgentType::Aider => "aider".to_string(),
            AgentType::Cline => "cline".to_string(),
            AgentType::Goose => "goose".to_string(),
            AgentType::Custom(name) => format!("custom_{}", name),
        }
    }
}

/// Status of an external agent process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentStatus {
    NotStarted,
    Starting,
    Running,
    Paused,
    Stopped,
    Crashed(String),
}

/// The kind of output line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LineType {
    Stdout,
    Stderr,
    System,
}

/// Content type for shared-context entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Code,
    FilePath,
    Json,
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

/// A single line of output captured from (or sent to) an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputLine {
    pub timestamp: u64,
    pub agent_id: String,
    pub line_type: LineType,
    pub content: String,
}

/// An entry on the shared-context clipboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextEntry {
    pub source_agent: String,
    pub content: String,
    pub content_type: ContentType,
    pub timestamp: u64,
}

/// Represents a single external agent managed by the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalAgent {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub status: AgentStatus,
    pub pid: Option<u32>,
    pub output_buffer: Vec<OutputLine>,
    pub max_buffer_lines: usize,
}

impl ExternalAgent {
    /// Create a new agent with sensible defaults.
    pub fn new(id: String, name: String, agent_type: AgentType, command: String, args: Vec<String>) -> Self {
        Self {
            id,
            name,
            agent_type,
            command,
            args,
            working_dir: None,
            env_vars: HashMap::new(),
            status: AgentStatus::NotStarted,
            pid: None,
            output_buffer: Vec::new(),
            max_buffer_lines: 1000,
        }
    }

    /// Append a line to the output buffer, truncating oldest lines when full.
    pub fn push_output(&mut self, line: OutputLine) {
        self.output_buffer.push(line);
        if self.output_buffer.len() > self.max_buffer_lines {
            let excess = self.output_buffer.len() - self.max_buffer_lines;
            self.output_buffer.drain(0..excess);
        }
    }
}

// ---------------------------------------------------------------------------
// SharedContext
// ---------------------------------------------------------------------------

/// A clipboard-style shared context that agents can push/pop entries to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedContext {
    pub clipboard: Vec<ContextEntry>,
    pub max_entries: usize,
}

impl SharedContext {
    pub fn new(max_entries: usize) -> Self {
        Self {
            clipboard: Vec::new(),
            max_entries,
        }
    }

    /// Push an entry, evicting the oldest if at capacity.
    pub fn push(&mut self, entry: ContextEntry) {
        if self.clipboard.len() >= self.max_entries {
            self.clipboard.remove(0);
        }
        self.clipboard.push(entry);
    }

    /// Pop the most-recently pushed entry.
    pub fn pop(&mut self) -> Option<ContextEntry> {
        self.clipboard.pop()
    }

    /// Peek at the most-recently pushed entry without removing it.
    pub fn peek(&self) -> Option<&ContextEntry> {
        self.clipboard.last()
    }

    /// List all entries.
    pub fn list(&self) -> &[ContextEntry] {
        &self.clipboard
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.clipboard.clear();
    }
}

// ---------------------------------------------------------------------------
// HostConfig
// ---------------------------------------------------------------------------

/// Configuration for the agent host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostConfig {
    pub max_agents: usize,
    pub default_working_dir: Option<String>,
    pub auto_start: bool,
    pub output_interleave: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            max_agents: 5,
            default_working_dir: None,
            auto_start: false,
            output_interleave: true,
        }
    }
}

// ---------------------------------------------------------------------------
// HostMetrics
// ---------------------------------------------------------------------------

/// Counters tracking host-wide activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostMetrics {
    pub total_registered: usize,
    pub total_started: usize,
    pub total_stopped: usize,
    pub total_messages_sent: usize,
    pub total_output_lines: usize,
    pub agents_by_type: HashMap<String, usize>,
}

impl HostMetrics {
    pub fn new() -> Self {
        Self {
            total_registered: 0,
            total_started: 0,
            total_stopped: 0,
            total_messages_sent: 0,
            total_output_lines: 0,
            agents_by_type: HashMap::new(),
        }
    }
}

impl Default for HostMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AgentHost
// ---------------------------------------------------------------------------

/// Central coordinator that owns all registered agents, shared context,
/// configuration, and metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentHost {
    pub agents: HashMap<String, ExternalAgent>,
    pub active_agent: Option<String>,
    pub shared_context: SharedContext,
    pub config: HostConfig,
    pub metrics: HostMetrics,
}

impl AgentHost {
    /// Create a new host with the given configuration.
    pub fn new(config: HostConfig) -> Self {
        Self {
            agents: HashMap::new(),
            active_agent: None,
            shared_context: SharedContext::new(100),
            config,
            metrics: HostMetrics::new(),
        }
    }

    /// Register a new agent. Returns the generated agent id.
    pub fn register_agent(
        &mut self,
        name: &str,
        agent_type: AgentType,
        command: &str,
        args: Vec<String>,
    ) -> Result<String, String> {
        if self.agents.len() >= self.config.max_agents {
            return Err(format!(
                "Max agents reached ({}). Cannot register more.",
                self.config.max_agents
            ));
        }

        // Check for duplicate names.
        for agent in self.agents.values() {
            if agent.name == name {
                return Err(format!("Agent with name '{}' already registered", name));
            }
        }

        let id = format!("agent_{}", self.metrics.total_registered + 1);
        let mut agent = ExternalAgent::new(
            id.clone(),
            name.to_string(),
            agent_type.clone(),
            command.to_string(),
            args,
        );

        if let Some(ref dir) = self.config.default_working_dir {
            agent.working_dir = Some(dir.clone());
        }

        self.agents.insert(id.clone(), agent);

        // Update metrics.
        self.metrics.total_registered += 1;
        let type_label = agent_type.label();
        *self.metrics.agents_by_type.entry(type_label).or_insert(0) += 1;

        // Auto-start if configured.
        if self.config.auto_start {
            let _ = self.start_agent(&id);
        }

        Ok(id)
    }

    /// Unregister an agent, stopping it first if running.
    pub fn unregister_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.status == AgentStatus::Running || agent.status == AgentStatus::Starting {
            // Stop first (ignore error — best-effort).
            let _ = self.stop_agent(id);
        }

        // If this was the active agent, clear.
        if self.active_agent.as_deref() == Some(id) {
            self.active_agent = None;
        }

        self.agents.remove(id);
        Ok(())
    }

    /// Simulated start: sets status to Running and assigns a fake PID.
    pub fn start_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        match &agent.status {
            AgentStatus::Running => return Err(format!("Agent '{}' is already running", id)),
            AgentStatus::Starting => return Err(format!("Agent '{}' is already starting", id)),
            _ => {}
        }

        agent.status = AgentStatus::Running;
        agent.pid = Some(10000 + self.metrics.total_started as u32);
        self.metrics.total_started += 1;

        // Record a system line.
        let line = OutputLine {
            timestamp: current_timestamp(),
            agent_id: id.to_string(),
            line_type: LineType::System,
            content: format!("Agent '{}' started", agent.name),
        };
        let agent = self.agents.get_mut(id).expect("agent exists");
        agent.push_output(line);
        self.metrics.total_output_lines += 1;

        Ok(())
    }

    /// Stop a running agent.
    pub fn stop_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.status != AgentStatus::Running && agent.status != AgentStatus::Paused {
            return Err(format!("Agent '{}' is not running (status: {:?})", id, agent.status));
        }

        agent.status = AgentStatus::Stopped;
        agent.pid = None;
        self.metrics.total_stopped += 1;

        let line = OutputLine {
            timestamp: current_timestamp(),
            agent_id: id.to_string(),
            line_type: LineType::System,
            content: format!("Agent '{}' stopped", agent.name),
        };
        let agent = self.agents.get_mut(id).expect("agent exists");
        agent.push_output(line);
        self.metrics.total_output_lines += 1;

        Ok(())
    }

    /// Send input text to an agent (recorded as a System output line).
    pub fn send_to_agent(&mut self, id: &str, input: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.status != AgentStatus::Running {
            return Err(format!("Agent '{}' is not running", id));
        }

        let line = OutputLine {
            timestamp: current_timestamp(),
            agent_id: id.to_string(),
            line_type: LineType::System,
            content: input.to_string(),
        };
        agent.push_output(line);
        self.metrics.total_messages_sent += 1;
        self.metrics.total_output_lines += 1;

        Ok(())
    }

    /// Return the last N output lines for a given agent.
    pub fn get_output(&self, id: &str, last_n: usize) -> Vec<OutputLine> {
        match self.agents.get(id) {
            Some(agent) => {
                let buf = &agent.output_buffer;
                if last_n >= buf.len() {
                    buf.clone()
                } else {
                    buf[buf.len() - last_n..].to_vec()
                }
            }
            None => Vec::new(),
        }
    }

    /// Set the currently-active (focused) agent.
    pub fn set_active(&mut self, id: &str) -> Result<(), String> {
        if !self.agents.contains_key(id) {
            return Err(format!("Agent '{}' not found", id));
        }
        self.active_agent = Some(id.to_string());
        Ok(())
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<&ExternalAgent> {
        self.agents.values().collect()
    }

    /// Count how many agents are currently Running.
    pub fn running_count(&self) -> usize {
        self.agents.values().filter(|a| a.status == AgentStatus::Running).count()
    }
}

// ---------------------------------------------------------------------------
// OutputRouter
// ---------------------------------------------------------------------------

/// Utilities for merging and filtering output across agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputRouter;

impl OutputRouter {
    /// Interleave output from multiple agents, sorted by timestamp, limited to last_n.
    pub fn interleave(agents: &[&ExternalAgent], last_n: usize) -> Vec<OutputLine> {
        let mut all: Vec<OutputLine> = agents
            .iter()
            .flat_map(|a| a.output_buffer.clone())
            .collect();
        all.sort_by_key(|l| l.timestamp);
        if last_n >= all.len() {
            all
        } else {
            all[all.len() - last_n..].to_vec()
        }
    }

    /// Filter lines to only those from a specific agent.
    pub fn filter_by_agent(lines: &[OutputLine], agent_id: &str) -> Vec<OutputLine> {
        lines.iter().filter(|l| l.agent_id == agent_id).cloned().collect()
    }

    /// Filter lines to only those of a specific line type.
    pub fn filter_by_type(lines: &[OutputLine], line_type: LineType) -> Vec<OutputLine> {
        lines.iter().filter(|l| l.line_type == line_type).cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// AgentSelector
// ---------------------------------------------------------------------------

/// Heuristic-based agent routing and suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSelector;

impl AgentSelector {
    /// Given a query string, find the best matching agent id by keyword matching
    /// against the agent type.
    pub fn route_by_capability(query: &str, agents: &[&ExternalAgent]) -> Option<String> {
        let q = query.to_lowercase();

        // Priority keywords per agent type.
        let scores: Vec<(String, usize)> = agents
            .iter()
            .map(|a| {
                let score = Self::keyword_score(&q, &a.agent_type);
                (a.id.clone(), score)
            })
            .collect();

        scores
            .into_iter()
            .filter(|(_, s)| *s > 0)
            .max_by_key(|(_, s)| *s)
            .map(|(id, _)| id)
    }

    /// Suggest the best agent type for a given task description.
    pub fn suggest_agent(task_description: &str) -> AgentType {
        let desc = task_description.to_lowercase();

        if desc.contains("refactor") || desc.contains("edit") || desc.contains("pair") {
            return AgentType::Aider;
        }
        if desc.contains("google") || desc.contains("gemini") || desc.contains("search") {
            return AgentType::GeminiCli;
        }
        if desc.contains("vscode") || desc.contains("ide") || desc.contains("extension") {
            return AgentType::Cline;
        }
        if desc.contains("automate") || desc.contains("pipeline") || desc.contains("goose") {
            return AgentType::Goose;
        }
        // Default
        AgentType::ClaudeCode
    }

    fn keyword_score(query: &str, agent_type: &AgentType) -> usize {
        let keywords: Vec<&str> = match agent_type {
            AgentType::ClaudeCode => vec![
                "claude", "anthropic", "code", "implement", "architect", "design", "complex",
            ],
            AgentType::GeminiCli => vec![
                "gemini", "google", "search", "research", "summarize", "analyze",
            ],
            AgentType::Aider => vec![
                "aider", "refactor", "edit", "pair", "diff", "patch", "git",
            ],
            AgentType::Cline => vec![
                "cline", "vscode", "ide", "extension", "debug", "ui",
            ],
            AgentType::Goose => vec![
                "goose", "automate", "pipeline", "workflow", "task", "script",
            ],
            AgentType::Custom(_) => vec![],
        };

        keywords.iter().filter(|kw| query.contains(**kw)).count()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Simple monotonic timestamp (seconds since epoch, or a counter for tests).
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_host() -> AgentHost {
        AgentHost::new(HostConfig::default())
    }

    fn make_output_line(ts: u64, agent_id: &str, lt: LineType, content: &str) -> OutputLine {
        OutputLine {
            timestamp: ts,
            agent_id: agent_id.to_string(),
            line_type: lt,
            content: content.to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Agent registration
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_claude_code_agent() {
        let mut host = default_host();
        let id = host
            .register_agent("my-claude", AgentType::ClaudeCode, "claude", vec![])
            .unwrap();
        assert_eq!(id, "agent_1");
        assert_eq!(host.agents.len(), 1);
        let agent = &host.agents[&id];
        assert_eq!(agent.name, "my-claude");
        assert_eq!(agent.agent_type, AgentType::ClaudeCode);
        assert_eq!(agent.status, AgentStatus::NotStarted);
    }

    #[test]
    fn test_register_gemini_agent() {
        let mut host = default_host();
        let id = host
            .register_agent("gem", AgentType::GeminiCli, "gemini", vec!["--model".into(), "pro".into()])
            .unwrap();
        let agent = &host.agents[&id];
        assert_eq!(agent.agent_type, AgentType::GeminiCli);
        assert_eq!(agent.args, vec!["--model", "pro"]);
    }

    #[test]
    fn test_register_aider_agent() {
        let mut host = default_host();
        let id = host.register_agent("aider1", AgentType::Aider, "aider", vec![]).unwrap();
        assert_eq!(host.agents[&id].agent_type, AgentType::Aider);
    }

    #[test]
    fn test_register_cline_agent() {
        let mut host = default_host();
        let id = host.register_agent("cline1", AgentType::Cline, "cline", vec![]).unwrap();
        assert_eq!(host.agents[&id].agent_type, AgentType::Cline);
    }

    #[test]
    fn test_register_goose_agent() {
        let mut host = default_host();
        let id = host.register_agent("goose1", AgentType::Goose, "goose", vec![]).unwrap();
        assert_eq!(host.agents[&id].agent_type, AgentType::Goose);
    }

    #[test]
    fn test_register_custom_agent() {
        let mut host = default_host();
        let id = host
            .register_agent("mybot", AgentType::Custom("MyBot".into()), "/usr/bin/mybot", vec![])
            .unwrap();
        assert_eq!(host.agents[&id].agent_type, AgentType::Custom("MyBot".into()));
    }

    #[test]
    fn test_register_duplicate_name_rejected() {
        let mut host = default_host();
        host.register_agent("dup", AgentType::ClaudeCode, "claude", vec![]).unwrap();
        let err = host
            .register_agent("dup", AgentType::Aider, "aider", vec![])
            .unwrap_err();
        assert!(err.contains("already registered"));
    }

    #[test]
    fn test_register_max_agents_exceeded() {
        let config = HostConfig { max_agents: 2, ..Default::default() };
        let mut host = AgentHost::new(config);
        host.register_agent("a1", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.register_agent("a2", AgentType::Aider, "a", vec![]).unwrap();
        let err = host.register_agent("a3", AgentType::Goose, "g", vec![]).unwrap_err();
        assert!(err.contains("Max agents reached"));
    }

    #[test]
    fn test_register_with_default_working_dir() {
        let config = HostConfig {
            default_working_dir: Some("/tmp/work".into()),
            ..Default::default()
        };
        let mut host = AgentHost::new(config);
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        assert_eq!(host.agents[&id].working_dir, Some("/tmp/work".into()));
    }

    // -----------------------------------------------------------------------
    // Agent lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn test_start_agent() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::Running);
        assert!(host.agents[&id].pid.is_some());
    }

    #[test]
    fn test_start_already_running() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        let err = host.start_agent(&id).unwrap_err();
        assert!(err.contains("already running"));
    }

    #[test]
    fn test_stop_agent() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        host.stop_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::Stopped);
        assert_eq!(host.agents[&id].pid, None);
    }

    #[test]
    fn test_stop_not_running() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        let err = host.stop_agent(&id).unwrap_err();
        assert!(err.contains("not running"));
    }

    #[test]
    fn test_lifecycle_register_start_stop() {
        let mut host = default_host();
        let id = host.register_agent("lifecycle", AgentType::Aider, "aider", vec![]).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::NotStarted);
        host.start_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::Running);
        host.stop_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::Stopped);
    }

    #[test]
    fn test_lifecycle_register_start_crash() {
        let mut host = default_host();
        let id = host.register_agent("crasher", AgentType::Goose, "goose", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        // Simulate crash externally.
        host.agents.get_mut(&id).unwrap().status =
            AgentStatus::Crashed("segfault".to_string());
        assert_eq!(
            host.agents[&id].status,
            AgentStatus::Crashed("segfault".to_string())
        );
    }

    #[test]
    fn test_unregister_agent() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.unregister_agent(&id).unwrap();
        assert!(host.agents.is_empty());
    }

    #[test]
    fn test_unregister_running_agent_stops_first() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        host.unregister_agent(&id).unwrap();
        assert!(host.agents.is_empty());
    }

    #[test]
    fn test_unregister_clears_active() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.set_active(&id).unwrap();
        host.unregister_agent(&id).unwrap();
        assert!(host.active_agent.is_none());
    }

    #[test]
    fn test_unregister_unknown_agent() {
        let mut host = default_host();
        let err = host.unregister_agent("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    // -----------------------------------------------------------------------
    // Output buffer
    // -----------------------------------------------------------------------

    #[test]
    fn test_send_to_agent_adds_output() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        host.send_to_agent(&id, "hello world").unwrap();
        let out = host.get_output(&id, 10);
        assert!(out.iter().any(|l| l.content == "hello world"));
    }

    #[test]
    fn test_send_to_not_running_fails() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        let err = host.send_to_agent(&id, "nope").unwrap_err();
        assert!(err.contains("not running"));
    }

    #[test]
    fn test_get_output_last_n() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap();
        for i in 0..5 {
            host.send_to_agent(&id, &format!("msg{}", i)).unwrap();
        }
        let out = host.get_output(&id, 2);
        assert_eq!(out.len(), 2);
        // Should be the last 2 messages.
        assert!(out[1].content.contains("msg4"));
    }

    #[test]
    fn test_get_output_unknown_agent_returns_empty() {
        let host = default_host();
        let out = host.get_output("no-such", 10);
        assert!(out.is_empty());
    }

    #[test]
    fn test_output_buffer_overflow_truncates() {
        let mut agent = ExternalAgent::new(
            "a".into(),
            "test".into(),
            AgentType::ClaudeCode,
            "c".into(),
            vec![],
        );
        agent.max_buffer_lines = 5;
        for i in 0..10 {
            agent.push_output(make_output_line(i, "a", LineType::Stdout, &format!("line{}", i)));
        }
        assert_eq!(agent.output_buffer.len(), 5);
        assert_eq!(agent.output_buffer[0].content, "line5");
        assert_eq!(agent.output_buffer[4].content, "line9");
    }

    // -----------------------------------------------------------------------
    // Shared context
    // -----------------------------------------------------------------------

    #[test]
    fn test_shared_context_push_pop() {
        let mut ctx = SharedContext::new(10);
        ctx.push(ContextEntry {
            source_agent: "a1".into(),
            content: "hello".into(),
            content_type: ContentType::Text,
            timestamp: 1,
        });
        let entry = ctx.pop().unwrap();
        assert_eq!(entry.content, "hello");
        assert!(ctx.pop().is_none());
    }

    #[test]
    fn test_shared_context_peek() {
        let mut ctx = SharedContext::new(10);
        assert!(ctx.peek().is_none());
        ctx.push(ContextEntry {
            source_agent: "a1".into(),
            content: "world".into(),
            content_type: ContentType::Code,
            timestamp: 2,
        });
        assert_eq!(ctx.peek().unwrap().content, "world");
        assert_eq!(ctx.list().len(), 1); // peek doesn't remove
    }

    #[test]
    fn test_shared_context_list_and_clear() {
        let mut ctx = SharedContext::new(10);
        for i in 0..3 {
            ctx.push(ContextEntry {
                source_agent: format!("a{}", i),
                content: format!("c{}", i),
                content_type: ContentType::FilePath,
                timestamp: i as u64,
            });
        }
        assert_eq!(ctx.list().len(), 3);
        ctx.clear();
        assert!(ctx.list().is_empty());
    }

    #[test]
    fn test_shared_context_eviction() {
        let mut ctx = SharedContext::new(2);
        for i in 0..4 {
            ctx.push(ContextEntry {
                source_agent: "a".into(),
                content: format!("c{}", i),
                content_type: ContentType::Json,
                timestamp: i as u64,
            });
        }
        assert_eq!(ctx.list().len(), 2);
        assert_eq!(ctx.list()[0].content, "c2");
        assert_eq!(ctx.list()[1].content, "c3");
    }

    // -----------------------------------------------------------------------
    // Output router
    // -----------------------------------------------------------------------

    #[test]
    fn test_interleave_merges_by_timestamp() {
        let mut a1 = ExternalAgent::new("a1".into(), "A1".into(), AgentType::ClaudeCode, "c".into(), vec![]);
        a1.push_output(make_output_line(1, "a1", LineType::Stdout, "first"));
        a1.push_output(make_output_line(3, "a1", LineType::Stdout, "third"));

        let mut a2 = ExternalAgent::new("a2".into(), "A2".into(), AgentType::Aider, "a".into(), vec![]);
        a2.push_output(make_output_line(2, "a2", LineType::Stdout, "second"));
        a2.push_output(make_output_line(4, "a2", LineType::Stdout, "fourth"));

        let merged = OutputRouter::interleave(&[&a1, &a2], 100);
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[0].content, "first");
        assert_eq!(merged[1].content, "second");
        assert_eq!(merged[2].content, "third");
        assert_eq!(merged[3].content, "fourth");
    }

    #[test]
    fn test_interleave_last_n() {
        let mut a1 = ExternalAgent::new("a1".into(), "A1".into(), AgentType::ClaudeCode, "c".into(), vec![]);
        for i in 0..10 {
            a1.push_output(make_output_line(i, "a1", LineType::Stdout, &format!("l{}", i)));
        }
        let merged = OutputRouter::interleave(&[&a1], 3);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].content, "l7");
    }

    #[test]
    fn test_filter_by_agent() {
        let lines = vec![
            make_output_line(1, "a1", LineType::Stdout, "x"),
            make_output_line(2, "a2", LineType::Stdout, "y"),
            make_output_line(3, "a1", LineType::Stderr, "z"),
        ];
        let filtered = OutputRouter::filter_by_agent(&lines, "a1");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|l| l.agent_id == "a1"));
    }

    #[test]
    fn test_filter_by_type() {
        let lines = vec![
            make_output_line(1, "a1", LineType::Stdout, "x"),
            make_output_line(2, "a1", LineType::Stderr, "y"),
            make_output_line(3, "a1", LineType::System, "z"),
            make_output_line(4, "a1", LineType::Stderr, "w"),
        ];
        let filtered = OutputRouter::filter_by_type(&lines, LineType::Stderr);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|l| l.line_type == LineType::Stderr));
    }

    // -----------------------------------------------------------------------
    // Agent selection and routing
    // -----------------------------------------------------------------------

    #[test]
    fn test_route_by_capability_claude() {
        let a1 = ExternalAgent::new("a1".into(), "Claude".into(), AgentType::ClaudeCode, "c".into(), vec![]);
        let a2 = ExternalAgent::new("a2".into(), "Aider".into(), AgentType::Aider, "a".into(), vec![]);
        let result = AgentSelector::route_by_capability("implement complex architecture", &[&a1, &a2]);
        assert_eq!(result, Some("a1".into()));
    }

    #[test]
    fn test_route_by_capability_aider() {
        let a1 = ExternalAgent::new("a1".into(), "Claude".into(), AgentType::ClaudeCode, "c".into(), vec![]);
        let a2 = ExternalAgent::new("a2".into(), "Aider".into(), AgentType::Aider, "a".into(), vec![]);
        let result = AgentSelector::route_by_capability("refactor and edit the diff", &[&a1, &a2]);
        assert_eq!(result, Some("a2".into()));
    }

    #[test]
    fn test_route_no_match() {
        let a1 = ExternalAgent::new("a1".into(), "C".into(), AgentType::Custom("x".into()), "c".into(), vec![]);
        let result = AgentSelector::route_by_capability("something obscure", &[&a1]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_suggest_agent_refactor() {
        assert_eq!(AgentSelector::suggest_agent("refactor this module"), AgentType::Aider);
    }

    #[test]
    fn test_suggest_agent_gemini() {
        assert_eq!(AgentSelector::suggest_agent("search google for docs"), AgentType::GeminiCli);
    }

    #[test]
    fn test_suggest_agent_cline() {
        assert_eq!(AgentSelector::suggest_agent("debug in vscode ide"), AgentType::Cline);
    }

    #[test]
    fn test_suggest_agent_goose() {
        assert_eq!(AgentSelector::suggest_agent("automate this pipeline"), AgentType::Goose);
    }

    #[test]
    fn test_suggest_agent_default_claude() {
        assert_eq!(AgentSelector::suggest_agent("write a new feature"), AgentType::ClaudeCode);
    }

    // -----------------------------------------------------------------------
    // Host config
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_host_config() {
        let cfg = HostConfig::default();
        assert_eq!(cfg.max_agents, 5);
        assert_eq!(cfg.default_working_dir, None);
        assert!(!cfg.auto_start);
        assert!(cfg.output_interleave);
    }

    #[test]
    fn test_auto_start_config() {
        let config = HostConfig { auto_start: true, ..Default::default() };
        let mut host = AgentHost::new(config);
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        assert_eq!(host.agents[&id].status, AgentStatus::Running);
    }

    // -----------------------------------------------------------------------
    // Metrics
    // -----------------------------------------------------------------------

    #[test]
    fn test_metrics_after_operations() {
        let mut host = default_host();
        let id1 = host.register_agent("a1", AgentType::ClaudeCode, "c", vec![]).unwrap();
        let id2 = host.register_agent("a2", AgentType::Aider, "a", vec![]).unwrap();
        host.start_agent(&id1).unwrap();
        host.start_agent(&id2).unwrap();
        host.send_to_agent(&id1, "hi").unwrap();
        host.send_to_agent(&id1, "bye").unwrap();
        host.stop_agent(&id2).unwrap();

        assert_eq!(host.metrics.total_registered, 2);
        assert_eq!(host.metrics.total_started, 2);
        assert_eq!(host.metrics.total_stopped, 1);
        assert_eq!(host.metrics.total_messages_sent, 2);
        assert_eq!(host.metrics.agents_by_type["claude_code"], 1);
        assert_eq!(host.metrics.agents_by_type["aider"], 1);
    }

    #[test]
    fn test_metrics_output_lines_tracked() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.start_agent(&id).unwrap(); // generates 1 system line
        host.send_to_agent(&id, "x").unwrap(); // generates 1 system line
        host.stop_agent(&id).unwrap(); // generates 1 system line
        // start line + send line + stop line = 3
        assert_eq!(host.metrics.total_output_lines, 3);
    }

    // -----------------------------------------------------------------------
    // Multiple concurrent agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_agents_running() {
        let mut host = default_host();
        let id1 = host.register_agent("a1", AgentType::ClaudeCode, "c", vec![]).unwrap();
        let id2 = host.register_agent("a2", AgentType::Aider, "a", vec![]).unwrap();
        let id3 = host.register_agent("a3", AgentType::Goose, "g", vec![]).unwrap();
        host.start_agent(&id1).unwrap();
        host.start_agent(&id2).unwrap();
        host.start_agent(&id3).unwrap();
        assert_eq!(host.running_count(), 3);
        host.stop_agent(&id2).unwrap();
        assert_eq!(host.running_count(), 2);
    }

    #[test]
    fn test_list_agents() {
        let mut host = default_host();
        host.register_agent("a1", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.register_agent("a2", AgentType::Aider, "a", vec![]).unwrap();
        assert_eq!(host.list_agents().len(), 2);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_active_unknown_agent() {
        let mut host = default_host();
        let err = host.set_active("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_set_active_and_verify() {
        let mut host = default_host();
        let id = host.register_agent("a", AgentType::ClaudeCode, "c", vec![]).unwrap();
        host.set_active(&id).unwrap();
        assert_eq!(host.active_agent, Some(id));
    }

    #[test]
    fn test_empty_host() {
        let host = default_host();
        assert!(host.list_agents().is_empty());
        assert_eq!(host.running_count(), 0);
        assert!(host.active_agent.is_none());
    }

    #[test]
    fn test_start_unknown_agent() {
        let mut host = default_host();
        let err = host.start_agent("ghost").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_stop_unknown_agent() {
        let mut host = default_host();
        let err = host.stop_agent("ghost").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_send_to_unknown_agent() {
        let mut host = default_host();
        let err = host.send_to_agent("ghost", "hi").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_agent_type_label() {
        assert_eq!(AgentType::ClaudeCode.label(), "claude_code");
        assert_eq!(AgentType::GeminiCli.label(), "gemini_cli");
        assert_eq!(AgentType::Aider.label(), "aider");
        assert_eq!(AgentType::Cline.label(), "cline");
        assert_eq!(AgentType::Goose.label(), "goose");
        assert_eq!(AgentType::Custom("Foo".into()).label(), "custom_Foo");
    }

    #[test]
    fn test_interleave_empty_agents() {
        let merged = OutputRouter::interleave(&[], 10);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_filter_by_agent_no_match() {
        let lines = vec![make_output_line(1, "a1", LineType::Stdout, "x")];
        let filtered = OutputRouter::filter_by_agent(&lines, "a99");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_by_type_system_only() {
        let lines = vec![
            make_output_line(1, "a1", LineType::Stdout, "a"),
            make_output_line(2, "a1", LineType::System, "b"),
        ];
        let filtered = OutputRouter::filter_by_type(&lines, LineType::System);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "b");
    }

    #[test]
    fn test_shared_context_content_types() {
        let mut ctx = SharedContext::new(10);
        ctx.push(ContextEntry {
            source_agent: "a".into(),
            content: "{}".into(),
            content_type: ContentType::Json,
            timestamp: 1,
        });
        ctx.push(ContextEntry {
            source_agent: "a".into(),
            content: "/tmp/foo.rs".into(),
            content_type: ContentType::FilePath,
            timestamp: 2,
        });
        assert_eq!(ctx.list().len(), 2);
        assert_eq!(ctx.list()[0].content_type, ContentType::Json);
        assert_eq!(ctx.list()[1].content_type, ContentType::FilePath);
    }

    #[test]
    fn test_host_metrics_default() {
        let m = HostMetrics::default();
        assert_eq!(m.total_registered, 0);
        assert_eq!(m.total_started, 0);
        assert_eq!(m.total_stopped, 0);
        assert_eq!(m.total_messages_sent, 0);
        assert_eq!(m.total_output_lines, 0);
        assert!(m.agents_by_type.is_empty());
    }

    #[test]
    fn test_external_agent_default_buffer_size() {
        let agent = ExternalAgent::new("id".into(), "n".into(), AgentType::ClaudeCode, "c".into(), vec![]);
        assert_eq!(agent.max_buffer_lines, 1000);
    }
}
