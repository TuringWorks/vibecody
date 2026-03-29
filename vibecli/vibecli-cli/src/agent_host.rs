//! Multi-agent terminal host — run external CLI agents alongside VibeCody.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Status of a hosted agent process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HostedAgentStatus {
    Starting,
    Running,
    Idle,
    Stopped,
    Crashed(String),
}

/// Label for an output line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputLabel {
    Stdout,
    Stderr,
    System,
}

/// Policy for routing messages to agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoutePolicy {
    RoundRobin,
    Broadcast,
    FirstMatch,
    Manual,
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

/// A single line of output captured from (or sent to) an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputLine {
    pub agent_id: String,
    pub label: OutputLabel,
    pub text: String,
    pub timestamp: u64,
}

/// Represents a single external agent managed by the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostedAgent {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub status: HostedAgentStatus,
    pub started_at: u64,
    pub output_lines: Vec<OutputLine>,
    pub env_vars: HashMap<String, String>,
}

/// An entry on the shared clipboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub key: String,
    pub value: String,
    pub source_agent: String,
    pub timestamp: u64,
}

/// Shared clipboard across all hosted agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedClipboard {
    pub entries: Vec<ClipboardEntry>,
    pub max_entries: usize,
}

impl SharedClipboard {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }
}

/// Configuration for the agent host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentHostConfig {
    pub max_agents: u32,
    pub route_policy: RoutePolicy,
    pub shared_clipboard_size: usize,
}

impl Default for AgentHostConfig {
    fn default() -> Self {
        Self {
            max_agents: 5,
            route_policy: RoutePolicy::RoundRobin,
            shared_clipboard_size: 100,
        }
    }
}

/// Counters tracking host-wide activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostMetrics {
    pub agents_launched: u64,
    pub agents_crashed: u64,
    pub total_output_lines: u64,
    pub clipboard_writes: u64,
}

impl HostMetrics {
    pub fn new() -> Self {
        Self {
            agents_launched: 0,
            agents_crashed: 0,
            total_output_lines: 0,
            clipboard_writes: 0,
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

/// Central coordinator that owns all registered agents, shared clipboard,
/// configuration, and metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentHost {
    pub config: AgentHostConfig,
    pub agents: HashMap<String, HostedAgent>,
    pub clipboard: SharedClipboard,
    pub metrics: HostMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl AgentHost {
    /// Create a new host with the given configuration.
    pub fn new(config: AgentHostConfig) -> Self {
        let clipboard_size = config.shared_clipboard_size;
        Self {
            config,
            agents: HashMap::new(),
            clipboard: SharedClipboard::new(clipboard_size),
            metrics: HostMetrics::new(),
            next_id: 1,
            timestamp_counter: 1,
        }
    }

    /// Return the next monotonic timestamp and advance the counter.
    fn next_ts(&mut self) -> u64 {
        let ts = self.timestamp_counter;
        self.timestamp_counter += 1;
        ts
    }

    /// Register and "launch" a new agent. Returns the generated agent id.
    pub fn add_agent(
        &mut self,
        name: &str,
        agent_type: &str,
        command: &str,
        args: Vec<String>,
    ) -> Result<String, String> {
        if self.agents.len() >= self.config.max_agents as usize {
            return Err(format!(
                "Max agents reached ({}). Cannot add more.",
                self.config.max_agents
            ));
        }

        let id = format!("agent_{}", self.next_id);
        self.next_id += 1;
        let ts = self.next_ts();

        let agent = HostedAgent {
            id: id.clone(),
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            command: command.to_string(),
            args,
            working_dir: ".".to_string(),
            status: HostedAgentStatus::Running,
            started_at: ts,
            output_lines: Vec::new(),
            env_vars: HashMap::new(),
        };

        self.agents.insert(id.clone(), agent);
        self.metrics.agents_launched += 1;

        Ok(id)
    }

    /// Remove an agent by id.
    pub fn remove_agent(&mut self, id: &str) -> Result<(), String> {
        if self.agents.remove(id).is_none() {
            return Err(format!("Agent '{}' not found", id));
        }
        Ok(())
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<&HostedAgent> {
        self.agents.values().collect()
    }

    /// Get a specific agent by id.
    pub fn get_agent(&self, id: &str) -> Option<&HostedAgent> {
        self.agents.get(id)
    }

    /// Stop a running agent (set status to Stopped).
    pub fn stop_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        match &agent.status {
            HostedAgentStatus::Stopped => {
                return Err(format!("Agent '{}' is already stopped", id));
            }
            HostedAgentStatus::Crashed(reason) => {
                return Err(format!("Agent '{}' has crashed: {}", id, reason));
            }
            _ => {}
        }

        agent.status = HostedAgentStatus::Stopped;
        Ok(())
    }

    /// Restart a stopped agent (set status back to Running).
    pub fn restart_agent(&mut self, id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        match &agent.status {
            HostedAgentStatus::Running => {
                return Err(format!("Agent '{}' is already running", id));
            }
            HostedAgentStatus::Starting => {
                return Err(format!("Agent '{}' is already starting", id));
            }
            _ => {}
        }

        let ts = self.timestamp_counter;
        self.timestamp_counter += 1;
        agent.status = HostedAgentStatus::Running;
        agent.started_at = ts;
        self.metrics.agents_launched += 1;
        Ok(())
    }

    /// Route a message to agents based on the configured policy.
    /// Returns the list of target agent IDs.
    pub fn route_message(&mut self, message: &str) -> Vec<String> {
        let running: Vec<String> = self
            .agents
            .values()
            .filter(|a| a.status == HostedAgentStatus::Running)
            .map(|a| a.id.clone())
            .collect();

        if running.is_empty() {
            return Vec::new();
        }

        let router = OutputRouter::new(self.config.route_policy.clone());
        let id_refs: Vec<&str> = running.iter().map(|s| s.as_str()).collect();
        router.route(&id_refs, message)
    }

    /// Send a message to a specific agent and get a simulated response.
    pub fn ask_agent(&mut self, id: &str, message: &str) -> Result<OutputLine, String> {
        let agent = self
            .agents
            .get(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        if agent.status != HostedAgentStatus::Running {
            return Err(format!("Agent '{}' is not running", id));
        }

        let agent_name = agent.name.clone();
        let ts = self.next_ts();
        let response_text = format!(
            "[{}] Response to: {}",
            agent_name, message
        );

        let output = OutputLine {
            agent_id: id.to_string(),
            label: OutputLabel::Stdout,
            text: response_text,
            timestamp: ts,
        };

        // Record in agent output
        let agent = self.agents.get_mut(id).expect("agent exists");
        agent.output_lines.push(output.clone());
        self.metrics.total_output_lines += 1;

        Ok(output)
    }

    /// Return the last N output lines for a given agent.
    pub fn get_output(&self, id: &str, limit: usize) -> Vec<&OutputLine> {
        match self.agents.get(id) {
            Some(agent) => {
                let lines = &agent.output_lines;
                if limit >= lines.len() {
                    lines.iter().collect()
                } else {
                    lines[lines.len() - limit..].iter().collect()
                }
            }
            None => Vec::new(),
        }
    }

    /// Return the last N output lines interleaved from all agents, sorted by timestamp.
    pub fn get_all_output(&self, limit: usize) -> Vec<&OutputLine> {
        let mut all: Vec<&OutputLine> = self
            .agents
            .values()
            .flat_map(|a| a.output_lines.iter())
            .collect();
        all.sort_by_key(|l| l.timestamp);
        if limit >= all.len() {
            all
        } else {
            all[all.len() - limit..].to_vec()
        }
    }

    /// Write an entry to the shared clipboard.
    pub fn write_clipboard(&mut self, key: &str, value: &str, source_agent: &str) {
        let ts = self.next_ts();
        let entry = ClipboardEntry {
            key: key.to_string(),
            value: value.to_string(),
            source_agent: source_agent.to_string(),
            timestamp: ts,
        };

        // Evict oldest if at capacity
        if self.clipboard.entries.len() >= self.clipboard.max_entries {
            self.clipboard.entries.remove(0);
        }

        self.clipboard.entries.push(entry);
        self.metrics.clipboard_writes += 1;
    }

    /// Read an entry from the shared clipboard by key.
    pub fn read_clipboard(&self, key: &str) -> Option<&ClipboardEntry> {
        self.clipboard
            .entries
            .iter()
            .rev()
            .find(|e| e.key == key)
    }

    /// List all clipboard entries.
    pub fn list_clipboard(&self) -> Vec<&ClipboardEntry> {
        self.clipboard.entries.iter().collect()
    }

    /// Return a reference to the host metrics.
    pub fn get_metrics(&self) -> &HostMetrics {
        &self.metrics
    }

    /// Count agents in Running or Starting status.
    pub fn active_count(&self) -> usize {
        self.agents
            .values()
            .filter(|a| {
                matches!(
                    a.status,
                    HostedAgentStatus::Running | HostedAgentStatus::Starting
                )
            })
            .count()
    }
}

// ---------------------------------------------------------------------------
// OutputRouter
// ---------------------------------------------------------------------------

/// Routes messages to agents based on the configured policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputRouter {
    policy: RoutePolicy,
}

impl OutputRouter {
    pub fn new(policy: RoutePolicy) -> Self {
        Self { policy }
    }

    /// Select agents based on the routing policy.
    pub fn route(&self, agents: &[&str], message: &str) -> Vec<String> {
        if agents.is_empty() {
            return Vec::new();
        }

        match &self.policy {
            RoutePolicy::RoundRobin => {
                // Pick one agent deterministically based on message length
                let idx = message.len() % agents.len();
                vec![agents[idx].to_string()]
            }
            RoutePolicy::Broadcast => {
                // Send to all agents
                agents.iter().map(|a| a.to_string()).collect()
            }
            RoutePolicy::FirstMatch => {
                // Pick the first agent
                vec![agents[0].to_string()]
            }
            RoutePolicy::Manual => {
                // Manual mode returns empty — caller must choose explicitly
                Vec::new()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_host() -> AgentHost {
        AgentHost::new(AgentHostConfig::default())
    }

    // -----------------------------------------------------------------------
    // Host creation and config
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_config() {
        let cfg = AgentHostConfig::default();
        assert_eq!(cfg.max_agents, 5);
        assert_eq!(cfg.route_policy, RoutePolicy::RoundRobin);
        assert_eq!(cfg.shared_clipboard_size, 100);
    }

    #[test]
    fn test_host_creation() {
        let host = default_host();
        assert!(host.agents.is_empty());
        assert_eq!(host.clipboard.max_entries, 100);
        assert_eq!(host.metrics.agents_launched, 0);
    }

    #[test]
    fn test_custom_config() {
        let cfg = AgentHostConfig {
            max_agents: 10,
            route_policy: RoutePolicy::Broadcast,
            shared_clipboard_size: 50,
        };
        let host = AgentHost::new(cfg);
        assert_eq!(host.config.max_agents, 10);
        assert_eq!(host.config.route_policy, RoutePolicy::Broadcast);
        assert_eq!(host.clipboard.max_entries, 50);
    }

    // -----------------------------------------------------------------------
    // Adding agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_add_agent_success() {
        let mut host = default_host();
        let id = host
            .add_agent("my-claude", "claude-code", "claude", vec![])
            .unwrap();
        assert_eq!(id, "agent_1");
        assert_eq!(host.agents.len(), 1);
        let agent = &host.agents[&id];
        assert_eq!(agent.name, "my-claude");
        assert_eq!(agent.agent_type, "claude-code");
        assert_eq!(agent.status, HostedAgentStatus::Running);
    }

    #[test]
    fn test_add_agent_with_args() {
        let mut host = default_host();
        let id = host
            .add_agent(
                "gem",
                "gemini-cli",
                "gemini",
                vec!["--model".into(), "pro".into()],
            )
            .unwrap();
        let agent = &host.agents[&id];
        assert_eq!(agent.args, vec!["--model", "pro"]);
    }

    #[test]
    fn test_add_agent_increments_id() {
        let mut host = default_host();
        let id1 = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        let id2 = host.add_agent("a2", "aider", "a", vec![]).unwrap();
        assert_eq!(id1, "agent_1");
        assert_eq!(id2, "agent_2");
    }

    #[test]
    fn test_add_agent_max_limit() {
        let cfg = AgentHostConfig {
            max_agents: 2,
            ..Default::default()
        };
        let mut host = AgentHost::new(cfg);
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        let err = host.add_agent("a3", "goose", "g", vec![]).unwrap_err();
        assert!(err.contains("Max agents reached"));
    }

    #[test]
    fn test_add_agent_updates_metrics() {
        let mut host = default_host();
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        assert_eq!(host.metrics.agents_launched, 2);
    }

    // -----------------------------------------------------------------------
    // Removing agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_agent_success() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.remove_agent(&id).unwrap();
        assert!(host.agents.is_empty());
    }

    #[test]
    fn test_remove_agent_not_found() {
        let mut host = default_host();
        let err = host.remove_agent("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    // -----------------------------------------------------------------------
    // Agent lifecycle: add → run → stop → restart → remove
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_lifecycle() {
        let mut host = default_host();
        let id = host.add_agent("lifecycle", "aider", "aider", vec![]).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Running);

        host.stop_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Stopped);

        host.restart_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Running);

        host.remove_agent(&id).unwrap();
        assert!(host.agents.is_empty());
    }

    #[test]
    fn test_stop_agent_success() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.stop_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Stopped);
    }

    #[test]
    fn test_stop_already_stopped() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.stop_agent(&id).unwrap();
        let err = host.stop_agent(&id).unwrap_err();
        assert!(err.contains("already stopped"));
    }

    #[test]
    fn test_stop_crashed_agent() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status =
            HostedAgentStatus::Crashed("segfault".to_string());
        let err = host.stop_agent(&id).unwrap_err();
        assert!(err.contains("crashed"));
    }

    #[test]
    fn test_stop_unknown_agent() {
        let mut host = default_host();
        let err = host.stop_agent("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_restart_agent_success() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.stop_agent(&id).unwrap();
        host.restart_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Running);
    }

    #[test]
    fn test_restart_already_running() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        let err = host.restart_agent(&id).unwrap_err();
        assert!(err.contains("already running"));
    }

    #[test]
    fn test_restart_unknown_agent() {
        let mut host = default_host();
        let err = host.restart_agent("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_restart_crashed_agent() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status =
            HostedAgentStatus::Crashed("oom".to_string());
        host.restart_agent(&id).unwrap();
        assert_eq!(host.agents[&id].status, HostedAgentStatus::Running);
    }

    #[test]
    fn test_restart_increments_launched() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        assert_eq!(host.metrics.agents_launched, 1);
        host.stop_agent(&id).unwrap();
        host.restart_agent(&id).unwrap();
        assert_eq!(host.metrics.agents_launched, 2);
    }

    // -----------------------------------------------------------------------
    // Message routing (all policies)
    // -----------------------------------------------------------------------

    #[test]
    fn test_route_round_robin() {
        let mut host = AgentHost::new(AgentHostConfig {
            route_policy: RoutePolicy::RoundRobin,
            ..Default::default()
        });
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        let targets = host.route_message("hello");
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_route_broadcast() {
        let mut host = AgentHost::new(AgentHostConfig {
            route_policy: RoutePolicy::Broadcast,
            ..Default::default()
        });
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        let targets = host.route_message("hello everyone");
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_route_first_match() {
        let mut host = AgentHost::new(AgentHostConfig {
            route_policy: RoutePolicy::FirstMatch,
            ..Default::default()
        });
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        let targets = host.route_message("hello");
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_route_manual_returns_empty() {
        let mut host = AgentHost::new(AgentHostConfig {
            route_policy: RoutePolicy::Manual,
            ..Default::default()
        });
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        let targets = host.route_message("hello");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_route_no_running_agents() {
        let mut host = default_host();
        let id = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.stop_agent(&id).unwrap();
        let targets = host.route_message("hello");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_route_skips_stopped_agents() {
        let mut host = AgentHost::new(AgentHostConfig {
            route_policy: RoutePolicy::Broadcast,
            ..Default::default()
        });
        let id1 = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        host.stop_agent(&id1).unwrap();
        let targets = host.route_message("hello");
        assert_eq!(targets.len(), 1);
        assert!(targets[0].contains("agent_"));
    }

    // -----------------------------------------------------------------------
    // Ask agent (simulated response)
    // -----------------------------------------------------------------------

    #[test]
    fn test_ask_agent_success() {
        let mut host = default_host();
        let id = host.add_agent("claude", "claude-code", "c", vec![]).unwrap();
        let output = host.ask_agent(&id, "what is 2+2?").unwrap();
        assert_eq!(output.agent_id, id);
        assert_eq!(output.label, OutputLabel::Stdout);
        assert!(output.text.contains("claude"));
        assert!(output.text.contains("what is 2+2?"));
    }

    #[test]
    fn test_ask_agent_not_found() {
        let mut host = default_host();
        let err = host.ask_agent("nope", "hello").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_ask_agent_not_running() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.stop_agent(&id).unwrap();
        let err = host.ask_agent(&id, "hello").unwrap_err();
        assert!(err.contains("not running"));
    }

    #[test]
    fn test_ask_agent_records_output() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.ask_agent(&id, "test").unwrap();
        assert_eq!(host.agents[&id].output_lines.len(), 1);
        assert_eq!(host.metrics.total_output_lines, 1);
    }

    // -----------------------------------------------------------------------
    // Output retrieval
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_output_per_agent() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        for i in 0..5 {
            host.ask_agent(&id, &format!("msg{}", i)).unwrap();
        }
        let out = host.get_output(&id, 3);
        assert_eq!(out.len(), 3);
        assert!(out[2].text.contains("msg4"));
    }

    #[test]
    fn test_get_output_limit_exceeds_total() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.ask_agent(&id, "hello").unwrap();
        let out = host.get_output(&id, 100);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn test_get_output_unknown_agent() {
        let host = default_host();
        let out = host.get_output("nope", 10);
        assert!(out.is_empty());
    }

    #[test]
    fn test_get_all_output_interleaved() {
        let mut host = default_host();
        let id1 = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        let id2 = host.add_agent("a2", "aider", "a", vec![]).unwrap();
        host.ask_agent(&id1, "first").unwrap();
        host.ask_agent(&id2, "second").unwrap();
        host.ask_agent(&id1, "third").unwrap();

        let all = host.get_all_output(100);
        assert_eq!(all.len(), 3);
        // Should be sorted by timestamp
        assert!(all[0].timestamp <= all[1].timestamp);
        assert!(all[1].timestamp <= all[2].timestamp);
    }

    #[test]
    fn test_get_all_output_limited() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        for i in 0..10 {
            host.ask_agent(&id, &format!("msg{}", i)).unwrap();
        }
        let all = host.get_all_output(3);
        assert_eq!(all.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Shared clipboard
    // -----------------------------------------------------------------------

    #[test]
    fn test_clipboard_write_and_read() {
        let mut host = default_host();
        host.write_clipboard("file", "/tmp/test.rs", "agent_1");
        let entry = host.read_clipboard("file").unwrap();
        assert_eq!(entry.value, "/tmp/test.rs");
        assert_eq!(entry.source_agent, "agent_1");
    }

    #[test]
    fn test_clipboard_read_not_found() {
        let host = default_host();
        assert!(host.read_clipboard("missing").is_none());
    }

    #[test]
    fn test_clipboard_read_returns_latest() {
        let mut host = default_host();
        host.write_clipboard("k", "old", "agent_1");
        host.write_clipboard("k", "new", "agent_2");
        let entry = host.read_clipboard("k").unwrap();
        assert_eq!(entry.value, "new");
        assert_eq!(entry.source_agent, "agent_2");
    }

    #[test]
    fn test_clipboard_list() {
        let mut host = default_host();
        host.write_clipboard("a", "1", "agent_1");
        host.write_clipboard("b", "2", "agent_2");
        let entries = host.list_clipboard();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_clipboard_overflow_eviction() {
        let cfg = AgentHostConfig {
            shared_clipboard_size: 3,
            ..Default::default()
        };
        let mut host = AgentHost::new(cfg);
        for i in 0..5 {
            host.write_clipboard(&format!("k{}", i), &format!("v{}", i), "a");
        }
        let entries = host.list_clipboard();
        assert_eq!(entries.len(), 3);
        // Oldest entries (k0, k1) should have been evicted
        assert_eq!(entries[0].key, "k2");
        assert_eq!(entries[1].key, "k3");
        assert_eq!(entries[2].key, "k4");
    }

    #[test]
    fn test_clipboard_writes_tracked_in_metrics() {
        let mut host = default_host();
        host.write_clipboard("a", "1", "agent_1");
        host.write_clipboard("b", "2", "agent_2");
        assert_eq!(host.metrics.clipboard_writes, 2);
    }

    // -----------------------------------------------------------------------
    // Metrics tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_metrics_initial_state() {
        let host = default_host();
        let m = host.get_metrics();
        assert_eq!(m.agents_launched, 0);
        assert_eq!(m.agents_crashed, 0);
        assert_eq!(m.total_output_lines, 0);
        assert_eq!(m.clipboard_writes, 0);
    }

    #[test]
    fn test_metrics_after_operations() {
        let mut host = default_host();
        let id = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.ask_agent(&id, "hi").unwrap();
        host.write_clipboard("k", "v", "a1");

        let m = host.get_metrics();
        assert_eq!(m.agents_launched, 1);
        assert_eq!(m.total_output_lines, 1);
        assert_eq!(m.clipboard_writes, 1);
    }

    #[test]
    fn test_metrics_default_impl() {
        let m = HostMetrics::default();
        assert_eq!(m.agents_launched, 0);
    }

    // -----------------------------------------------------------------------
    // Active count
    // -----------------------------------------------------------------------

    #[test]
    fn test_active_count() {
        let mut host = default_host();
        let id1 = host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        assert_eq!(host.active_count(), 2);
        host.stop_agent(&id1).unwrap();
        assert_eq!(host.active_count(), 1);
    }

    #[test]
    fn test_active_count_includes_starting() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status = HostedAgentStatus::Starting;
        assert_eq!(host.active_count(), 1);
    }

    #[test]
    fn test_active_count_excludes_idle() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status = HostedAgentStatus::Idle;
        assert_eq!(host.active_count(), 0);
    }

    // -----------------------------------------------------------------------
    // List / get agents
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_agents() {
        let mut host = default_host();
        host.add_agent("a1", "claude-code", "c", vec![]).unwrap();
        host.add_agent("a2", "aider", "a", vec![]).unwrap();
        assert_eq!(host.list_agents().len(), 2);
    }

    #[test]
    fn test_get_agent_found() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        let agent = host.get_agent(&id).unwrap();
        assert_eq!(agent.name, "a");
    }

    #[test]
    fn test_get_agent_not_found() {
        let host = default_host();
        assert!(host.get_agent("nope").is_none());
    }

    // -----------------------------------------------------------------------
    // OutputRouter
    // -----------------------------------------------------------------------

    #[test]
    fn test_router_round_robin_single() {
        let router = OutputRouter::new(RoutePolicy::RoundRobin);
        let result = router.route(&["a1", "a2", "a3"], "hi");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_router_broadcast() {
        let router = OutputRouter::new(RoutePolicy::Broadcast);
        let result = router.route(&["a1", "a2", "a3"], "hello");
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"a1".to_string()));
        assert!(result.contains(&"a2".to_string()));
        assert!(result.contains(&"a3".to_string()));
    }

    #[test]
    fn test_router_first_match() {
        let router = OutputRouter::new(RoutePolicy::FirstMatch);
        let result = router.route(&["a1", "a2"], "hello");
        assert_eq!(result, vec!["a1"]);
    }

    #[test]
    fn test_router_manual_returns_empty() {
        let router = OutputRouter::new(RoutePolicy::Manual);
        let result = router.route(&["a1", "a2"], "hello");
        assert!(result.is_empty());
    }

    #[test]
    fn test_router_empty_agents() {
        let router = OutputRouter::new(RoutePolicy::Broadcast);
        let result = router.route(&[], "hello");
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_stop_unknown() {
        let mut host = default_host();
        assert!(host.stop_agent("unknown").is_err());
    }

    #[test]
    fn test_error_restart_unknown() {
        let mut host = default_host();
        assert!(host.restart_agent("unknown").is_err());
    }

    #[test]
    fn test_error_ask_unknown() {
        let mut host = default_host();
        assert!(host.ask_agent("unknown", "hi").is_err());
    }

    #[test]
    fn test_error_remove_unknown() {
        let mut host = default_host();
        assert!(host.remove_agent("unknown").is_err());
    }

    #[test]
    fn test_error_restart_already_starting() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status = HostedAgentStatus::Starting;
        let err = host.restart_agent(&id).unwrap_err();
        assert!(err.contains("already starting"));
    }

    #[test]
    fn test_crashed_status_with_reason() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        host.agents.get_mut(&id).unwrap().status =
            HostedAgentStatus::Crashed("out of memory".to_string());
        if let HostedAgentStatus::Crashed(reason) = &host.agents[&id].status {
            assert_eq!(reason, "out of memory");
        } else {
            panic!("Expected Crashed status");
        }
    }

    #[test]
    fn test_env_vars_on_agent() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        let agent = host.agents.get_mut(&id).unwrap();
        agent.env_vars.insert("API_KEY".to_string(), "secret".to_string());
        assert_eq!(agent.env_vars["API_KEY"], "secret");
    }

    #[test]
    fn test_working_dir_default() {
        let mut host = default_host();
        let id = host.add_agent("a", "claude-code", "c", vec![]).unwrap();
        assert_eq!(host.agents[&id].working_dir, ".");
    }
}
