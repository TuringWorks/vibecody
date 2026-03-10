#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    Lead,
    Teammate,
    Observer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    WaitingForInput,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    TaskAssignment,
    StatusUpdate,
    PeerRequest,
    PeerResponse,
    Broadcast,
    DirectMessage,
    Escalation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAgent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub context: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub from_id: String,
    pub to_id: Option<String>,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTask {
    pub id: String,
    pub description: String,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
    pub priority: u8,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTaskList {
    pub tasks: Vec<SharedTask>,
}

impl SharedTaskList {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add(&mut self, task: SharedTask) {
        self.tasks.push(task);
    }

    pub fn remove(&mut self, task_id: &str) -> bool {
        let len_before = self.tasks.len();
        self.tasks.retain(|t| t.id != task_id);
        self.tasks.len() < len_before
    }

    pub fn assign(&mut self, task_id: &str, agent_id: &str) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.assigned_to = Some(agent_id.to_string());
            task.status = TaskStatus::InProgress;
            true
        } else {
            false
        }
    }

    pub fn complete(&mut self, task_id: &str) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Completed;
            true
        } else {
            false
        }
    }

    pub fn get(&self, task_id: &str) -> Option<&SharedTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStatus {
    pub total_agents: usize,
    pub leads: usize,
    pub teammates: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub messages_sent: usize,
}

pub struct AgentTeamManager {
    agents: HashMap<String, TeamAgent>,
    messages: Vec<TeamMessage>,
    task_list: SharedTaskList,
    next_agent_id: u64,
    next_task_id: u64,
    timestamp_counter: u64,
}

impl AgentTeamManager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            messages: Vec::new(),
            task_list: SharedTaskList::new(),
            next_agent_id: 1,
            next_task_id: 1,
            timestamp_counter: 0,
        }
    }

    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_counter += 1;
        self.timestamp_counter
    }

    pub fn add_agent(&mut self, name: &str, role: AgentRole, capabilities: Vec<String>) -> String {
        let id = format!("agent-{}", self.next_agent_id);
        self.next_agent_id += 1;
        let agent = TeamAgent {
            id: id.clone(),
            name: name.to_string(),
            role,
            context: String::new(),
            capabilities,
            status: AgentStatus::Idle,
        };
        self.agents.insert(id.clone(), agent);
        id
    }

    pub fn assign_lead(&mut self, agent_id: &str) -> bool {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.role = AgentRole::Lead;
            true
        } else {
            false
        }
    }

    pub fn send_message(
        &mut self,
        from: &str,
        to: &str,
        content: &str,
        msg_type: MessageType,
    ) -> bool {
        if !self.agents.contains_key(from) || !self.agents.contains_key(to) {
            return false;
        }
        let ts = self.next_timestamp();
        self.messages.push(TeamMessage {
            from_id: from.to_string(),
            to_id: Some(to.to_string()),
            content: content.to_string(),
            message_type: msg_type,
            timestamp: ts,
        });
        true
    }

    pub fn broadcast(&mut self, from: &str, content: &str) -> bool {
        if !self.agents.contains_key(from) {
            return false;
        }
        let ts = self.next_timestamp();
        self.messages.push(TeamMessage {
            from_id: from.to_string(),
            to_id: None,
            content: content.to_string(),
            message_type: MessageType::Broadcast,
            timestamp: ts,
        });
        true
    }

    pub fn get_messages(&self, agent_id: &str) -> Vec<TeamMessage> {
        self.messages
            .iter()
            .filter(|m| {
                // Messages sent directly to this agent, or broadcasts not from this agent
                match &m.to_id {
                    Some(to) => to == agent_id,
                    None => m.from_id != agent_id, // broadcasts received by everyone except sender
                }
            })
            .cloned()
            .collect()
    }

    pub fn delegate_task(
        &mut self,
        lead_id: &str,
        teammate_id: &str,
        description: &str,
    ) -> Option<String> {
        // Verify lead is actually a Lead
        let is_lead = self
            .agents
            .get(lead_id)
            .map(|a| a.role == AgentRole::Lead)
            .unwrap_or(false);
        if !is_lead || !self.agents.contains_key(teammate_id) {
            return None;
        }

        let task_id = format!("task-{}", self.next_task_id);
        self.next_task_id += 1;

        let task = SharedTask {
            id: task_id.clone(),
            description: description.to_string(),
            assigned_to: Some(teammate_id.to_string()),
            status: TaskStatus::InProgress,
            priority: 1,
            dependencies: Vec::new(),
        };
        self.task_list.add(task);

        // Update teammate status
        if let Some(agent) = self.agents.get_mut(teammate_id) {
            agent.status = AgentStatus::Working;
        }

        // Send task assignment message
        let ts = self.next_timestamp();
        self.messages.push(TeamMessage {
            from_id: lead_id.to_string(),
            to_id: Some(teammate_id.to_string()),
            content: format!("Task assigned: {}", description),
            message_type: MessageType::TaskAssignment,
            timestamp: ts,
        });

        Some(task_id)
    }

    pub fn escalate(&mut self, from: &str, content: &str) -> bool {
        if !self.agents.contains_key(from) {
            return false;
        }

        // Find the lead(s)
        let leads: Vec<String> = self
            .agents
            .values()
            .filter(|a| a.role == AgentRole::Lead)
            .map(|a| a.id.clone())
            .collect();

        if leads.is_empty() {
            return false;
        }

        for lead_id in &leads {
            let ts = self.next_timestamp();
            self.messages.push(TeamMessage {
                from_id: from.to_string(),
                to_id: Some(lead_id.clone()),
                content: content.to_string(),
                message_type: MessageType::Escalation,
                timestamp: ts,
            });
        }
        true
    }

    pub fn get_team_status(&self) -> TeamStatus {
        let leads = self
            .agents
            .values()
            .filter(|a| a.role == AgentRole::Lead)
            .count();
        let teammates = self
            .agents
            .values()
            .filter(|a| a.role == AgentRole::Teammate)
            .count();
        let active_tasks = self
            .task_list
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::InProgress || t.status == TaskStatus::Pending)
            .count();
        let completed_tasks = self
            .task_list
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();

        TeamStatus {
            total_agents: self.agents.len(),
            leads,
            teammates,
            active_tasks,
            completed_tasks,
            messages_sent: self.messages.len(),
        }
    }

    pub fn get_task_list(&self) -> &SharedTaskList {
        &self.task_list
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<&TeamAgent> {
        self.agents.get(agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_manager() {
        let mgr = AgentTeamManager::new();
        assert_eq!(mgr.agents.len(), 0);
        assert_eq!(mgr.messages.len(), 0);
    }

    #[test]
    fn test_add_agent() {
        let mut mgr = AgentTeamManager::new();
        let id = mgr.add_agent("Alice", AgentRole::Lead, vec!["rust".into()]);
        assert_eq!(id, "agent-1");
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.name, "Alice");
        assert_eq!(agent.role, AgentRole::Lead);
    }

    #[test]
    fn test_add_multiple_agents() {
        let mut mgr = AgentTeamManager::new();
        let id1 = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let id2 = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert_ne!(id1, id2);
        assert_eq!(mgr.get_team_status().total_agents, 2);
    }

    #[test]
    fn test_agent_capabilities() {
        let mut mgr = AgentTeamManager::new();
        let id = mgr.add_agent("Alice", AgentRole::Lead, vec!["rust".into(), "python".into()]);
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.capabilities.len(), 2);
        assert!(agent.capabilities.contains(&"rust".to_string()));
    }

    #[test]
    fn test_agent_initial_status() {
        let mut mgr = AgentTeamManager::new();
        let id = mgr.add_agent("Alice", AgentRole::Teammate, vec![]);
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.status, AgentStatus::Idle);
    }

    #[test]
    fn test_assign_lead() {
        let mut mgr = AgentTeamManager::new();
        let id = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert_eq!(mgr.get_agent(&id).unwrap().role, AgentRole::Teammate);
        assert!(mgr.assign_lead(&id));
        assert_eq!(mgr.get_agent(&id).unwrap().role, AgentRole::Lead);
    }

    #[test]
    fn test_assign_lead_nonexistent() {
        let mut mgr = AgentTeamManager::new();
        assert!(!mgr.assign_lead("no-such-agent"));
    }

    #[test]
    fn test_send_message() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(mgr.send_message(&a, &b, "hello", MessageType::DirectMessage));
        let inbox = mgr.get_messages(&b);
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].content, "hello");
        assert_eq!(inbox[0].message_type, MessageType::DirectMessage);
    }

    #[test]
    fn test_send_message_invalid_sender() {
        let mut mgr = AgentTeamManager::new();
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(!mgr.send_message("ghost", &b, "hi", MessageType::DirectMessage));
    }

    #[test]
    fn test_send_message_invalid_receiver() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        assert!(!mgr.send_message(&a, "ghost", "hi", MessageType::DirectMessage));
    }

    #[test]
    fn test_peer_request_response() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Teammate, vec![]);
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        mgr.send_message(&a, &b, "need help", MessageType::PeerRequest);
        mgr.send_message(&b, &a, "here you go", MessageType::PeerResponse);
        let a_inbox = mgr.get_messages(&a);
        assert_eq!(a_inbox.len(), 1);
        assert_eq!(a_inbox[0].message_type, MessageType::PeerResponse);
    }

    #[test]
    fn test_broadcast() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        let c = mgr.add_agent("Carol", AgentRole::Teammate, vec![]);
        assert!(mgr.broadcast(&a, "team update"));
        // Bob and Carol receive, Alice does not
        assert_eq!(mgr.get_messages(&b).len(), 1);
        assert_eq!(mgr.get_messages(&c).len(), 1);
        assert_eq!(mgr.get_messages(&a).len(), 0);
    }

    #[test]
    fn test_broadcast_invalid_sender() {
        let mut mgr = AgentTeamManager::new();
        assert!(!mgr.broadcast("ghost", "hello"));
    }

    #[test]
    fn test_delegate_task() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        let task_id = mgr.delegate_task(&lead, &tm, "implement feature X");
        assert!(task_id.is_some());
        let tid = task_id.unwrap();
        let task = mgr.get_task_list().get(&tid).unwrap();
        assert_eq!(task.description, "implement feature X");
        assert_eq!(task.assigned_to.as_deref(), Some(tm.as_str()));
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_delegate_task_non_lead_fails() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Teammate, vec![]);
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(mgr.delegate_task(&a, &b, "task").is_none());
    }

    #[test]
    fn test_delegate_task_updates_teammate_status() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        mgr.delegate_task(&lead, &tm, "do work");
        assert_eq!(mgr.get_agent(&tm).unwrap().status, AgentStatus::Working);
    }

    #[test]
    fn test_delegate_task_sends_message() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        mgr.delegate_task(&lead, &tm, "do work");
        let inbox = mgr.get_messages(&tm);
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].message_type, MessageType::TaskAssignment);
    }

    #[test]
    fn test_escalate() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(mgr.escalate(&tm, "I'm blocked"));
        let lead_inbox = mgr.get_messages(&lead);
        assert_eq!(lead_inbox.len(), 1);
        assert_eq!(lead_inbox[0].message_type, MessageType::Escalation);
        assert_eq!(lead_inbox[0].content, "I'm blocked");
    }

    #[test]
    fn test_escalate_no_lead() {
        let mut mgr = AgentTeamManager::new();
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(!mgr.escalate(&tm, "help"));
    }

    #[test]
    fn test_escalate_invalid_sender() {
        let mut mgr = AgentTeamManager::new();
        mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        assert!(!mgr.escalate("ghost", "help"));
    }

    #[test]
    fn test_team_status() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm1 = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        mgr.add_agent("Carol", AgentRole::Observer, vec![]);
        mgr.delegate_task(&lead, &tm1, "task 1");
        let status = mgr.get_team_status();
        assert_eq!(status.total_agents, 3);
        assert_eq!(status.leads, 1);
        assert_eq!(status.teammates, 1);
        assert_eq!(status.active_tasks, 1);
        assert_eq!(status.messages_sent, 1); // delegation sends a message
    }

    #[test]
    fn test_shared_task_list_add_remove() {
        let mut list = SharedTaskList::new();
        list.add(SharedTask {
            id: "t1".into(),
            description: "task one".into(),
            assigned_to: None,
            status: TaskStatus::Pending,
            priority: 1,
            dependencies: vec![],
        });
        assert_eq!(list.tasks.len(), 1);
        assert!(list.remove("t1"));
        assert_eq!(list.tasks.len(), 0);
        assert!(!list.remove("t1")); // already removed
    }

    #[test]
    fn test_shared_task_list_assign() {
        let mut list = SharedTaskList::new();
        list.add(SharedTask {
            id: "t1".into(),
            description: "task".into(),
            assigned_to: None,
            status: TaskStatus::Pending,
            priority: 1,
            dependencies: vec![],
        });
        assert!(list.assign("t1", "agent-1"));
        let task = list.get("t1").unwrap();
        assert_eq!(task.assigned_to.as_deref(), Some("agent-1"));
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_shared_task_list_complete() {
        let mut list = SharedTaskList::new();
        list.add(SharedTask {
            id: "t1".into(),
            description: "task".into(),
            assigned_to: None,
            status: TaskStatus::InProgress,
            priority: 1,
            dependencies: vec![],
        });
        assert!(list.complete("t1"));
        assert_eq!(list.get("t1").unwrap().status, TaskStatus::Completed);
    }

    #[test]
    fn test_shared_task_list_assign_nonexistent() {
        let mut list = SharedTaskList::new();
        assert!(!list.assign("nope", "agent-1"));
    }

    #[test]
    fn test_completed_tasks_in_status() {
        let mut mgr = AgentTeamManager::new();
        let lead = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        let tid = mgr.delegate_task(&lead, &tm, "task").unwrap();
        mgr.task_list.complete(&tid);
        let status = mgr.get_team_status();
        assert_eq!(status.completed_tasks, 1);
        assert_eq!(status.active_tasks, 0);
    }

    #[test]
    fn test_message_timestamps_increment() {
        let mut mgr = AgentTeamManager::new();
        let a = mgr.add_agent("Alice", AgentRole::Lead, vec![]);
        let b = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        mgr.send_message(&a, &b, "first", MessageType::DirectMessage);
        mgr.send_message(&a, &b, "second", MessageType::DirectMessage);
        let inbox = mgr.get_messages(&b);
        assert!(inbox[1].timestamp > inbox[0].timestamp);
    }

    #[test]
    fn test_observer_role() {
        let mut mgr = AgentTeamManager::new();
        let obs = mgr.add_agent("Observer", AgentRole::Observer, vec![]);
        let agent = mgr.get_agent(&obs).unwrap();
        assert_eq!(agent.role, AgentRole::Observer);
        // Observer cannot delegate
        let tm = mgr.add_agent("Bob", AgentRole::Teammate, vec![]);
        assert!(mgr.delegate_task(&obs, &tm, "task").is_none());
    }

    #[test]
    fn test_shared_task_dependencies() {
        let mut list = SharedTaskList::new();
        list.add(SharedTask {
            id: "t1".into(),
            description: "base task".into(),
            assigned_to: None,
            status: TaskStatus::Pending,
            priority: 1,
            dependencies: vec![],
        });
        list.add(SharedTask {
            id: "t2".into(),
            description: "depends on t1".into(),
            assigned_to: None,
            status: TaskStatus::Pending,
            priority: 2,
            dependencies: vec!["t1".into()],
        });
        let t2 = list.get("t2").unwrap();
        assert_eq!(t2.dependencies, vec!["t1".to_string()]);
    }
}
