//! Agent Teams — peer-to-peer communication between agents.
//!
//! Agents within a team share a broadcast message bus. The team lead decomposes
//! the task and assigns sub-tasks. Members can share findings, challenge
//! assumptions, and request information from each other via structured messages.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

// ── Team Message ─────────────────────────────────────────────────────────────

/// Type of message exchanged between team agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamMessageType {
    /// Agent sharing a discovery or result.
    Finding,
    /// Agent questioning or challenging another agent's approach.
    Challenge,
    /// Agent requesting information or help from other agents.
    Request,
    /// Status update (progress, completion, error).
    Status,
    /// Task assignment from the team lead.
    TaskAssignment,
    /// Acknowledgment of received message.
    Ack,
}

/// A message exchanged on the team bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    /// ID of the sending agent.
    pub from_agent_id: String,
    /// Optional target agent ID. `None` = broadcast to all.
    pub to_agent_id: Option<String>,
    /// Message type for filtering.
    pub msg_type: TeamMessageType,
    /// The message content.
    pub content: String,
    /// Timestamp (epoch ms).
    pub timestamp: u64,
}

impl TeamMessage {
    pub fn new(from: &str, msg_type: TeamMessageType, content: &str) -> Self {
        Self {
            from_agent_id: from.to_string(),
            to_agent_id: None,
            msg_type,
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn directed(from: &str, to: &str, msg_type: TeamMessageType, content: &str) -> Self {
        let mut msg = Self::new(from, msg_type, content);
        msg.to_agent_id = Some(to.to_string());
        msg
    }
}

// ── Team Message Bus ─────────────────────────────────────────────────────────

/// Shared broadcast channel for inter-agent communication.
#[derive(Clone)]
pub struct TeamMessageBus {
    sender: broadcast::Sender<TeamMessage>,
    /// Full message history for late joiners.
    history: Arc<Mutex<Vec<TeamMessage>>>,
}

impl std::fmt::Debug for TeamMessageBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamMessageBus").finish()
    }
}

impl TeamMessageBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Broadcast a message to all subscribers.
    pub async fn send(&self, message: TeamMessage) -> Result<(), String> {
        {
            let mut history = self.history.lock().await;
            history.push(message.clone());
        }
        let _ = self.sender.send(message);
        Ok(())
    }

    /// Subscribe to the bus. Returns a receiver for new messages.
    pub fn subscribe(&self) -> broadcast::Receiver<TeamMessage> {
        self.sender.subscribe()
    }

    /// Get full message history.
    pub async fn history(&self) -> Vec<TeamMessage> {
        self.history.lock().await.clone()
    }

    /// Get messages relevant to a specific agent (broadcast + directed).
    pub async fn messages_for(&self, agent_id: &str) -> Vec<TeamMessage> {
        let history = self.history.lock().await;
        history
            .iter()
            .filter(|m| {
                m.from_agent_id != agent_id
                    && (m.to_agent_id.is_none()
                        || m.to_agent_id.as_deref() == Some(agent_id))
            })
            .cloned()
            .collect()
    }

    /// Count total messages.
    pub async fn message_count(&self) -> usize {
        self.history.lock().await.len()
    }
}

// ── Team Task Decomposition ──────────────────────────────────────────────────

/// A sub-task assigned by the team lead to a member agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSubTask {
    pub id: String,
    pub agent_id: String,
    pub description: String,
    pub status: TeamTaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamTaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

// ── Agent Team ───────────────────────────────────────────────────────────────

/// A team of agents that can communicate and collaborate.
#[derive(Clone)]
pub struct AgentTeam {
    /// Team identifier.
    pub id: String,
    /// ID of the team lead agent.
    pub lead_agent_id: String,
    /// IDs of member agents (includes lead).
    pub member_ids: Vec<String>,
    /// Shared message bus.
    pub bus: TeamMessageBus,
    /// Task decomposition managed by the lead.
    pub tasks: Arc<Mutex<Vec<TeamSubTask>>>,
    /// Overall team goal.
    pub goal: String,
    /// Team status: "forming" | "working" | "complete" | "failed"
    pub status: Arc<Mutex<String>>,
}

impl AgentTeam {
    pub fn new(id: &str, lead_id: &str, goal: &str) -> Self {
        Self {
            id: id.to_string(),
            lead_agent_id: lead_id.to_string(),
            member_ids: vec![lead_id.to_string()],
            bus: TeamMessageBus::new(256),
            tasks: Arc::new(Mutex::new(Vec::new())),
            goal: goal.to_string(),
            status: Arc::new(Mutex::new("forming".to_string())),
        }
    }

    /// Add a member to the team.
    pub fn add_member(&mut self, agent_id: &str) {
        if !self.member_ids.contains(&agent_id.to_string()) {
            self.member_ids.push(agent_id.to_string());
        }
    }

    /// Set the task decomposition.
    pub async fn set_tasks(&self, tasks: Vec<TeamSubTask>) {
        let mut t = self.tasks.lock().await;
        *t = tasks;
    }

    /// Update the status of a sub-task.
    pub async fn update_task_status(&self, task_id: &str, status: TeamTaskStatus, result: Option<String>) {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = status;
            if result.is_some() {
                task.result = result;
            }
        }
    }

    /// Check if all sub-tasks are complete.
    pub async fn all_complete(&self) -> bool {
        let tasks = self.tasks.lock().await;
        !tasks.is_empty() && tasks.iter().all(|t| {
            t.status == TeamTaskStatus::Completed || t.status == TeamTaskStatus::Failed
        })
    }

    /// Get summary of team progress.
    pub async fn progress_summary(&self) -> String {
        let tasks = self.tasks.lock().await;
        let total = tasks.len();
        let done = tasks.iter().filter(|t| t.status == TeamTaskStatus::Completed).count();
        let failed = tasks.iter().filter(|t| t.status == TeamTaskStatus::Failed).count();
        let in_progress = tasks.iter().filter(|t| t.status == TeamTaskStatus::InProgress).count();
        format!("{}/{} complete, {} in progress, {} failed", done, total, in_progress, failed)
    }

    /// Set team status.
    pub async fn set_status(&self, s: &str) {
        let mut status = self.status.lock().await;
        *status = s.to_string();
    }

    /// Get team status.
    pub async fn get_status(&self) -> String {
        self.status.lock().await.clone()
    }
}

// ── Team Info (serializable snapshot) ────────────────────────────────────────

/// Serializable snapshot of team state for UI/API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    pub id: String,
    pub lead_agent_id: String,
    pub member_ids: Vec<String>,
    pub goal: String,
    pub status: String,
    pub tasks: Vec<TeamSubTask>,
    pub message_count: usize,
}

impl AgentTeam {
    pub async fn to_info(&self) -> TeamInfo {
        TeamInfo {
            id: self.id.clone(),
            lead_agent_id: self.lead_agent_id.clone(),
            member_ids: self.member_ids.clone(),
            goal: self.goal.clone(),
            status: self.get_status().await,
            tasks: self.tasks.lock().await.clone(),
            message_count: self.bus.message_count().await,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn team_message_broadcast() {
        let bus = TeamMessageBus::new(16);
        let mut rx = bus.subscribe();

        let msg = TeamMessage::new("agent-1", TeamMessageType::Finding, "Found a bug in auth.rs");
        bus.send(msg.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.from_agent_id, "agent-1");
        assert_eq!(received.content, "Found a bug in auth.rs");
        assert_eq!(received.msg_type, TeamMessageType::Finding);
    }

    #[tokio::test]
    async fn team_message_directed() {
        let bus = TeamMessageBus::new(16);

        let msg = TeamMessage::directed("agent-1", "agent-2", TeamMessageType::Request, "Need the API schema");
        bus.send(msg).await.unwrap();

        let for_2 = bus.messages_for("agent-2").await;
        assert_eq!(for_2.len(), 1);
        assert_eq!(for_2[0].content, "Need the API schema");

        let for_3 = bus.messages_for("agent-3").await;
        assert_eq!(for_3.len(), 0);
    }

    #[tokio::test]
    async fn team_message_history() {
        let bus = TeamMessageBus::new(16);

        bus.send(TeamMessage::new("a1", TeamMessageType::Status, "started")).await.unwrap();
        bus.send(TeamMessage::new("a2", TeamMessageType::Finding, "found issue")).await.unwrap();

        let history = bus.history().await;
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn team_lifecycle() {
        let mut team = AgentTeam::new("team-1", "lead", "Fix all bugs");
        team.add_member("worker-1");
        team.add_member("worker-2");

        assert_eq!(team.member_ids.len(), 3);

        team.set_tasks(vec![
            TeamSubTask {
                id: "t1".into(), agent_id: "worker-1".into(),
                description: "Fix auth bug".into(),
                status: TeamTaskStatus::Pending, result: None,
            },
            TeamSubTask {
                id: "t2".into(), agent_id: "worker-2".into(),
                description: "Fix API bug".into(),
                status: TeamTaskStatus::Pending, result: None,
            },
        ]).await;

        assert!(!team.all_complete().await);
        team.update_task_status("t1", TeamTaskStatus::Completed, Some("Fixed".into())).await;
        assert!(!team.all_complete().await);
        team.update_task_status("t2", TeamTaskStatus::Completed, Some("Fixed".into())).await;
        assert!(team.all_complete().await);
    }

    #[tokio::test]
    async fn team_progress_summary() {
        let team = AgentTeam::new("team-1", "lead", "goal");
        team.set_tasks(vec![
            TeamSubTask { id: "t1".into(), agent_id: "a".into(), description: "d".into(), status: TeamTaskStatus::Completed, result: None },
            TeamSubTask { id: "t2".into(), agent_id: "b".into(), description: "d".into(), status: TeamTaskStatus::InProgress, result: None },
            TeamSubTask { id: "t3".into(), agent_id: "c".into(), description: "d".into(), status: TeamTaskStatus::Failed, result: None },
        ]).await;

        let summary = team.progress_summary().await;
        assert!(summary.contains("1/3 complete"));
        assert!(summary.contains("1 in progress"));
        assert!(summary.contains("1 failed"));
    }

    #[tokio::test]
    async fn team_info_snapshot() {
        let team = AgentTeam::new("t1", "lead", "goal");
        team.set_status("working").await;
        let info = team.to_info().await;
        assert_eq!(info.id, "t1");
        assert_eq!(info.status, "working");
        assert_eq!(info.member_ids, vec!["lead"]);
    }

    #[test]
    fn team_message_new() {
        let msg = TeamMessage::new("a1", TeamMessageType::Challenge, "Why REST?");
        assert_eq!(msg.from_agent_id, "a1");
        assert!(msg.to_agent_id.is_none());
        assert!(msg.timestamp > 0);
    }

    #[tokio::test]
    async fn messages_for_excludes_self() {
        let bus = TeamMessageBus::new(16);

        bus.send(TeamMessage::new("agent-1", TeamMessageType::Finding, "my finding")).await.unwrap();
        bus.send(TeamMessage::new("agent-2", TeamMessageType::Finding, "their finding")).await.unwrap();

        let for_1 = bus.messages_for("agent-1").await;
        assert_eq!(for_1.len(), 1);
        assert_eq!(for_1[0].from_agent_id, "agent-2");
    }

    #[tokio::test]
    async fn duplicate_member_not_added() {
        let mut team = AgentTeam::new("t", "lead", "g");
        team.add_member("lead");
        team.add_member("worker");
        team.add_member("worker");
        assert_eq!(team.member_ids.len(), 2);
    }

    // ── TeamMessage serde roundtrip ──────────────────────────────────────

    #[test]
    fn team_message_serde_roundtrip() {
        let msg = TeamMessage::new("agent-1", TeamMessageType::Finding, "Found an issue");
        let json = serde_json::to_string(&msg).unwrap();
        let back: TeamMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from_agent_id, "agent-1");
        assert_eq!(back.content, "Found an issue");
        assert_eq!(back.msg_type, TeamMessageType::Finding);
        assert!(back.to_agent_id.is_none());
    }

    #[test]
    fn team_message_directed_serde_roundtrip() {
        let msg = TeamMessage::directed("a1", "a2", TeamMessageType::Request, "help");
        let json = serde_json::to_string(&msg).unwrap();
        let back: TeamMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.to_agent_id.as_deref(), Some("a2"));
        assert_eq!(back.msg_type, TeamMessageType::Request);
    }

    // ── TeamMessageType serde all variants ───────────────────────────────

    #[test]
    fn team_message_type_serde_all_variants() {
        let variants = vec![
            TeamMessageType::Finding,
            TeamMessageType::Challenge,
            TeamMessageType::Request,
            TeamMessageType::Status,
            TeamMessageType::TaskAssignment,
            TeamMessageType::Ack,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let back: TeamMessageType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // ── TeamSubTask serde ────────────────────────────────────────────────

    #[test]
    fn team_sub_task_serde_roundtrip() {
        let task = TeamSubTask {
            id: "t1".into(),
            agent_id: "worker-1".into(),
            description: "Fix auth bug".into(),
            status: TeamTaskStatus::InProgress,
            result: Some("Working on it".into()),
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: TeamSubTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "t1");
        assert_eq!(back.agent_id, "worker-1");
        assert_eq!(back.status, TeamTaskStatus::InProgress);
        assert_eq!(back.result.as_deref(), Some("Working on it"));
    }

    // ── TeamTaskStatus serde ─────────────────────────────────────────────

    #[test]
    fn team_task_status_serde_all_variants() {
        let variants = vec![
            TeamTaskStatus::Pending,
            TeamTaskStatus::InProgress,
            TeamTaskStatus::Completed,
            TeamTaskStatus::Failed,
        ];
        for status in variants {
            let json = serde_json::to_string(&status).unwrap();
            let back: TeamTaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    // ── TeamInfo serde ───────────────────────────────────────────────────

    #[test]
    fn team_info_serde_roundtrip() {
        let info = TeamInfo {
            id: "team-1".into(),
            lead_agent_id: "lead".into(),
            member_ids: vec!["lead".into(), "worker-1".into()],
            goal: "Fix all bugs".into(),
            status: "working".into(),
            tasks: vec![TeamSubTask {
                id: "t1".into(),
                agent_id: "worker-1".into(),
                description: "Fix auth".into(),
                status: TeamTaskStatus::Completed,
                result: Some("Fixed".into()),
            }],
            message_count: 5,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: TeamInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "team-1");
        assert_eq!(back.member_ids.len(), 2);
        assert_eq!(back.tasks.len(), 1);
        assert_eq!(back.message_count, 5);
    }

    // ── AgentTeam status ─────────────────────────────────────────────────

    #[tokio::test]
    async fn team_default_status_is_forming() {
        let team = AgentTeam::new("t", "lead", "goal");
        assert_eq!(team.get_status().await, "forming");
    }

    #[tokio::test]
    async fn team_set_and_get_status() {
        let team = AgentTeam::new("t", "lead", "goal");
        team.set_status("complete").await;
        assert_eq!(team.get_status().await, "complete");
    }

    // ── all_complete with empty tasks ────────────────────────────────────

    #[tokio::test]
    async fn all_complete_empty_tasks_returns_false() {
        let team = AgentTeam::new("t", "lead", "goal");
        // Empty tasks should not be considered "all complete"
        assert!(!team.all_complete().await);
    }

    // ── update_task_status with nonexistent task ─────────────────────────

    #[tokio::test]
    async fn update_task_status_nonexistent_is_no_op() {
        let team = AgentTeam::new("t", "lead", "goal");
        team.set_tasks(vec![TeamSubTask {
            id: "t1".into(),
            agent_id: "a".into(),
            description: "d".into(),
            status: TeamTaskStatus::Pending,
            result: None,
        }]).await;
        // Should not panic
        team.update_task_status("nonexistent", TeamTaskStatus::Completed, None).await;
        let tasks = team.tasks.lock().await;
        assert_eq!(tasks[0].status, TeamTaskStatus::Pending);
    }

    // ── progress_summary edge case: empty tasks ──────────────────────────

    #[tokio::test]
    async fn progress_summary_empty() {
        let team = AgentTeam::new("t", "lead", "goal");
        let summary = team.progress_summary().await;
        assert!(summary.contains("0/0 complete"));
    }

    // ── TeamMessage construction ────────────────────────────────────────

    #[test]
    fn team_message_timestamp_is_positive() {
        let msg = TeamMessage::new("a", TeamMessageType::Status, "up");
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn team_message_directed_has_to_field() {
        let msg = TeamMessage::directed("a1", "a2", TeamMessageType::Ack, "ok");
        assert_eq!(msg.from_agent_id, "a1");
        assert_eq!(msg.to_agent_id.as_deref(), Some("a2"));
        assert_eq!(msg.msg_type, TeamMessageType::Ack);
    }

    #[test]
    fn team_message_broadcast_has_no_to_field() {
        let msg = TeamMessage::new("a1", TeamMessageType::Finding, "found it");
        assert!(msg.to_agent_id.is_none());
    }

    // ── TeamMessageBus edge cases ───────────────────────────────────────

    #[tokio::test]
    async fn message_bus_empty_history() {
        let bus = TeamMessageBus::new(16);
        assert_eq!(bus.message_count().await, 0);
        assert!(bus.history().await.is_empty());
    }

    #[tokio::test]
    async fn message_bus_messages_for_broadcast_visible_to_others() {
        let bus = TeamMessageBus::new(16);
        bus.send(TeamMessage::new("a1", TeamMessageType::Finding, "broadcast msg")).await.unwrap();
        // Broadcast should be visible to a2 but not a1 (self excluded)
        let for_a2 = bus.messages_for("a2").await;
        assert_eq!(for_a2.len(), 1);
        let for_a1 = bus.messages_for("a1").await;
        assert_eq!(for_a1.len(), 0);
    }

    #[tokio::test]
    async fn message_bus_directed_not_visible_to_third_party() {
        let bus = TeamMessageBus::new(16);
        bus.send(TeamMessage::directed("a1", "a2", TeamMessageType::Request, "private")).await.unwrap();
        let for_a3 = bus.messages_for("a3").await;
        assert_eq!(for_a3.len(), 0, "directed message should not be visible to third party");
    }

    #[tokio::test]
    async fn message_bus_count_matches_sends() {
        let bus = TeamMessageBus::new(16);
        for i in 0..5 {
            bus.send(TeamMessage::new(&format!("a{}", i), TeamMessageType::Status, "msg")).await.unwrap();
        }
        assert_eq!(bus.message_count().await, 5);
    }

    // ── AgentTeam construction ──────────────────────────────────────────

    #[test]
    fn team_new_lead_is_first_member() {
        let team = AgentTeam::new("t1", "lead-agent", "build feature");
        assert_eq!(team.id, "t1");
        assert_eq!(team.lead_agent_id, "lead-agent");
        assert_eq!(team.member_ids, vec!["lead-agent"]);
        assert_eq!(team.goal, "build feature");
    }

    #[test]
    fn team_add_multiple_unique_members() {
        let mut team = AgentTeam::new("t", "lead", "g");
        team.add_member("w1");
        team.add_member("w2");
        team.add_member("w3");
        assert_eq!(team.member_ids.len(), 4); // lead + 3 workers
    }

    // ── TeamSubTask ─────────────────────────────────────────────────────

    #[test]
    fn team_sub_task_with_no_result() {
        let task = TeamSubTask {
            id: "t1".into(),
            agent_id: "a1".into(),
            description: "do work".into(),
            status: TeamTaskStatus::Pending,
            result: None,
        };
        assert!(task.result.is_none());
        assert_eq!(task.status, TeamTaskStatus::Pending);
    }

    #[test]
    fn team_sub_task_clone() {
        let task = TeamSubTask {
            id: "t1".into(),
            agent_id: "a1".into(),
            description: "work".into(),
            status: TeamTaskStatus::InProgress,
            result: Some("partial".into()),
        };
        let cloned = task.clone();
        assert_eq!(cloned.id, "t1");
        assert_eq!(cloned.result.as_deref(), Some("partial"));
    }

    // ── TeamTaskStatus equality ─────────────────────────────────────────

    #[test]
    fn team_task_status_ne() {
        assert_ne!(TeamTaskStatus::Pending, TeamTaskStatus::Completed);
        assert_ne!(TeamTaskStatus::InProgress, TeamTaskStatus::Failed);
    }

    // ── TeamInfo construction ───────────────────────────────────────────

    #[test]
    fn team_info_empty_tasks() {
        let info = TeamInfo {
            id: "t1".into(),
            lead_agent_id: "lead".into(),
            member_ids: vec!["lead".into()],
            goal: "test".into(),
            status: "forming".into(),
            tasks: vec![],
            message_count: 0,
        };
        assert!(info.tasks.is_empty());
        assert_eq!(info.message_count, 0);
    }
}
