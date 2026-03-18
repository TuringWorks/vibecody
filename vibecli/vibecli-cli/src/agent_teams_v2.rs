//! Agent Teams v2 — peer-to-peer messaging with shared task list.
//!
//! Closes P1 Gap 4: Teammates message each other directly, shared task list
//! with real-time status, lead synthesizes with conflict resolution.
//!
//! Extends the existing `agent_team.rs` (inter-agent messaging bus) with:
//! - Direct peer messaging between teammates
//! - Shared task board with status tracking
//! - Conflict detection when multiple agents edit the same file
//! - Lead agent synthesis with resolution strategies

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Agent identity
// ---------------------------------------------------------------------------

/// Unique identifier for a team member agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Role an agent plays on the team.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Lead,
    Teammate,
    Reviewer,
    Specialist(String),
}

impl AgentRole {
    pub fn name(&self) -> &str {
        match self {
            AgentRole::Lead => "lead",
            AgentRole::Teammate => "teammate",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Specialist(name) => name,
        }
    }
}

/// A team member with identity and capabilities.
#[derive(Debug, Clone)]
pub struct TeamMember {
    pub id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub status: MemberStatus,
    pub capabilities: Vec<String>,
    pub joined_at: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberStatus {
    Idle,
    Working,
    WaitingForPeer,
    Done,
    Error(String),
}

impl TeamMember {
    pub fn new(id: &str, name: &str, role: AgentRole) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: AgentId::new(id),
            name: name.to_string(),
            role,
            status: MemberStatus::Idle,
            capabilities: Vec::new(),
            joined_at: ts,
        }
    }

    pub fn with_capability(mut self, cap: &str) -> Self {
        self.capabilities.push(cap.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Peer messaging
// ---------------------------------------------------------------------------

/// A message sent between team members.
#[derive(Debug, Clone)]
pub struct PeerMessage {
    pub id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub reply_to: Option<String>,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    /// Regular text message
    Text,
    /// Request for help/input
    Request,
    /// Response to a request
    Response,
    /// Status update
    StatusUpdate,
    /// File change notification
    FileChange { path: String },
    /// Conflict alert
    ConflictAlert { file: String },
    /// Task assignment
    TaskAssignment { task_id: String },
}

impl PeerMessage {
    pub fn text(from: &str, to: &str, content: &str) -> Self {
        Self::new(from, to, content, MessageType::Text)
    }

    pub fn request(from: &str, to: &str, content: &str) -> Self {
        Self::new(from, to, content, MessageType::Request)
    }

    pub fn response(from: &str, to: &str, content: &str, reply_to: &str) -> Self {
        let mut msg = Self::new(from, to, content, MessageType::Response);
        msg.reply_to = Some(reply_to.to_string());
        msg
    }

    fn new(from: &str, to: &str, content: &str, msg_type: MessageType) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: format!("msg-{}", ts),
            from: AgentId::new(from),
            to: AgentId::new(to),
            content: content.to_string(),
            message_type: msg_type,
            timestamp: ts,
            reply_to: None,
            read: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared task board
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked(String),
    InReview,
    Complete,
    Failed(String),
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Blocked(_) => "blocked",
            TaskStatus::InReview => "in_review",
            TaskStatus::Complete => "complete",
            TaskStatus::Failed(_) => "failed",
        }
    }
}

/// A task on the shared board.
#[derive(Debug, Clone)]
pub struct SharedTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub assignee: Option<AgentId>,
    pub created_by: AgentId,
    pub created_at: u64,
    pub updated_at: u64,
    pub files_touched: Vec<String>,
    pub dependencies: Vec<String>,
    pub output: Option<String>,
}

impl SharedTask {
    pub fn new(id: &str, title: &str, created_by: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            status: TaskStatus::Pending,
            assignee: None,
            created_by: AgentId::new(created_by),
            created_at: ts,
            updated_at: ts,
            files_touched: Vec::new(),
            dependencies: Vec::new(),
            output: None,
        }
    }

    pub fn assign(&mut self, agent_id: &str) {
        self.assignee = Some(AgentId::new(agent_id));
        self.status = TaskStatus::InProgress;
        self.updated_at = now();
    }

    pub fn complete(&mut self, output: &str) {
        self.status = TaskStatus::Complete;
        self.output = Some(output.to_string());
        self.updated_at = now();
    }

    pub fn block(&mut self, reason: &str) {
        self.status = TaskStatus::Blocked(reason.to_string());
        self.updated_at = now();
    }

    pub fn touch_file(&mut self, path: &str) {
        if !self.files_touched.contains(&path.to_string()) {
            self.files_touched.push(path.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// Conflict detection
// ---------------------------------------------------------------------------

/// A file conflict between two agents.
#[derive(Debug, Clone)]
pub struct FileConflict {
    pub file: String,
    pub agent_a: AgentId,
    pub agent_b: AgentId,
    pub task_a: String,
    pub task_b: String,
    pub resolved: bool,
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    /// Agent A's changes take priority
    KeepA,
    /// Agent B's changes take priority
    KeepB,
    /// Merge both changes
    Merge,
    /// Lead agent manually resolved
    LeadResolved(String),
}

// ---------------------------------------------------------------------------
// Team coordinator
// ---------------------------------------------------------------------------

/// Central coordinator for an agent team.
pub struct TeamCoordinator {
    members: HashMap<String, TeamMember>,
    messages: Vec<PeerMessage>,
    tasks: Vec<SharedTask>,
    conflicts: Vec<FileConflict>,
    message_counter: u64,
}

impl TeamCoordinator {
    pub fn new() -> Self {
        Self {
            members: HashMap::new(),
            messages: Vec::new(),
            tasks: Vec::new(),
            conflicts: Vec::new(),
            message_counter: 0,
        }
    }

    // -- Members --

    pub fn add_member(&mut self, member: TeamMember) {
        self.members.insert(member.id.0.clone(), member);
    }

    pub fn remove_member(&mut self, id: &str) -> bool {
        self.members.remove(id).is_some()
    }

    pub fn get_member(&self, id: &str) -> Option<&TeamMember> {
        self.members.get(id)
    }

    pub fn get_member_mut(&mut self, id: &str) -> Option<&mut TeamMember> {
        self.members.get_mut(id)
    }

    pub fn list_members(&self) -> Vec<&TeamMember> {
        self.members.values().collect()
    }

    pub fn lead(&self) -> Option<&TeamMember> {
        self.members.values().find(|m| m.role == AgentRole::Lead)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    // -- Messaging --

    pub fn send_message(&mut self, mut msg: PeerMessage) {
        self.message_counter += 1;
        msg.id = format!("msg-{}", self.message_counter);
        self.messages.push(msg);
    }

    pub fn send_to_peer(&mut self, from: &str, to: &str, content: &str) {
        let msg = PeerMessage::text(from, to, content);
        self.send_message(msg);
    }

    pub fn broadcast(&mut self, from: &str, content: &str) {
        let recipients: Vec<String> = self
            .members
            .keys()
            .filter(|k| *k != from)
            .cloned()
            .collect();
        for to in recipients {
            self.send_to_peer(from, &to, content);
        }
    }

    pub fn inbox(&self, agent_id: &str) -> Vec<&PeerMessage> {
        self.messages
            .iter()
            .filter(|m| m.to.0 == agent_id)
            .collect()
    }

    pub fn unread_count(&self, agent_id: &str) -> usize {
        self.messages
            .iter()
            .filter(|m| m.to.0 == agent_id && !m.read)
            .count()
    }

    pub fn mark_read(&mut self, agent_id: &str) {
        for msg in &mut self.messages {
            if msg.to.0 == agent_id {
                msg.read = true;
            }
        }
    }

    pub fn conversation(&self, agent_a: &str, agent_b: &str) -> Vec<&PeerMessage> {
        self.messages
            .iter()
            .filter(|m| {
                (m.from.0 == agent_a && m.to.0 == agent_b)
                    || (m.from.0 == agent_b && m.to.0 == agent_a)
            })
            .collect()
    }

    // -- Tasks --

    pub fn add_task(&mut self, task: SharedTask) {
        self.tasks.push(task);
    }

    pub fn get_task(&self, id: &str) -> Option<&SharedTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut SharedTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn assign_task(&mut self, task_id: &str, agent_id: &str) -> bool {
        if let Some(task) = self.get_task_mut(task_id) {
            task.assign(agent_id);
            true
        } else {
            false
        }
    }

    pub fn complete_task(&mut self, task_id: &str, output: &str) -> bool {
        if let Some(task) = self.get_task_mut(task_id) {
            task.complete(output);
            true
        } else {
            false
        }
    }

    pub fn tasks_by_status(&self, status_str: &str) -> Vec<&SharedTask> {
        self.tasks
            .iter()
            .filter(|t| t.status.as_str() == status_str)
            .collect()
    }

    pub fn tasks_for_agent(&self, agent_id: &str) -> Vec<&SharedTask> {
        self.tasks
            .iter()
            .filter(|t| t.assignee.as_ref().is_some_and(|a| a.0 == agent_id))
            .collect()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    // -- Conflict detection --

    /// Check for file conflicts across all in-progress tasks.
    pub fn detect_conflicts(&mut self) -> Vec<FileConflict> {
        let mut file_owners: HashMap<String, (String, String)> = HashMap::new(); // file -> (agent_id, task_id)
        let mut new_conflicts = Vec::new();

        for task in &self.tasks {
            if task.status != TaskStatus::InProgress {
                continue;
            }
            let agent_id = match &task.assignee {
                Some(a) => a.0.clone(),
                None => continue,
            };
            for file in &task.files_touched {
                if let Some((other_agent, other_task)) = file_owners.get(file) {
                    if *other_agent != agent_id {
                        new_conflicts.push(FileConflict {
                            file: file.clone(),
                            agent_a: AgentId::new(other_agent),
                            agent_b: AgentId::new(&agent_id),
                            task_a: other_task.clone(),
                            task_b: task.id.clone(),
                            resolved: false,
                            resolution: None,
                        });
                    }
                } else {
                    file_owners.insert(file.clone(), (agent_id.clone(), task.id.clone()));
                }
            }
        }

        self.conflicts.extend(new_conflicts.clone());
        new_conflicts
    }

    pub fn resolve_conflict(&mut self, file: &str, resolution: ConflictResolution) -> bool {
        for conflict in &mut self.conflicts {
            if conflict.file == file && !conflict.resolved {
                conflict.resolved = true;
                conflict.resolution = Some(resolution);
                return true;
            }
        }
        false
    }

    pub fn unresolved_conflicts(&self) -> Vec<&FileConflict> {
        self.conflicts.iter().filter(|c| !c.resolved).collect()
    }

    // -- Synthesis --

    /// Lead synthesizes all completed task outputs.
    pub fn synthesize(&self) -> SynthesisReport {
        let completed: Vec<&SharedTask> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Complete)
            .collect();
        let pending: Vec<&SharedTask> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::InProgress)
            .collect();
        let conflicts = self.unresolved_conflicts();

        let mut all_files: Vec<String> = completed
            .iter()
            .flat_map(|t| t.files_touched.iter().cloned())
            .collect();
        all_files.sort();
        all_files.dedup();

        SynthesisReport {
            total_tasks: self.tasks.len(),
            completed_count: completed.len(),
            pending_count: pending.len(),
            conflict_count: conflicts.len(),
            files_modified: all_files,
            outputs: completed
                .iter()
                .map(|t| (t.id.clone(), t.output.clone().unwrap_or_default()))
                .collect(),
        }
    }

    pub fn stats(&self) -> TeamStats {
        TeamStats {
            member_count: self.members.len(),
            total_messages: self.messages.len(),
            total_tasks: self.tasks.len(),
            completed_tasks: self.tasks.iter().filter(|t| t.status == TaskStatus::Complete).count(),
            active_tasks: self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count(),
            unresolved_conflicts: self.conflicts.iter().filter(|c| !c.resolved).count(),
        }
    }
}

impl Default for TeamCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SynthesisReport {
    pub total_tasks: usize,
    pub completed_count: usize,
    pub pending_count: usize,
    pub conflict_count: usize,
    pub files_modified: Vec<String>,
    pub outputs: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct TeamStats {
    pub member_count: usize,
    pub total_messages: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub active_tasks: usize,
    pub unresolved_conflicts: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_team() -> TeamCoordinator {
        let mut team = TeamCoordinator::new();
        team.add_member(TeamMember::new("lead", "Alice", AgentRole::Lead));
        team.add_member(TeamMember::new("dev1", "Bob", AgentRole::Teammate));
        team.add_member(TeamMember::new("dev2", "Carol", AgentRole::Teammate));
        team
    }

    #[test]
    fn test_agent_id() {
        let id = AgentId::new("a1");
        assert_eq!(id.0, "a1");
    }

    #[test]
    fn test_agent_role_name() {
        assert_eq!(AgentRole::Lead.name(), "lead");
        assert_eq!(AgentRole::Teammate.name(), "teammate");
        assert_eq!(AgentRole::Reviewer.name(), "reviewer");
        assert_eq!(AgentRole::Specialist("dba".into()).name(), "dba");
    }

    #[test]
    fn test_team_member() {
        let m = TeamMember::new("a1", "Alice", AgentRole::Lead)
            .with_capability("rust")
            .with_capability("testing");
        assert_eq!(m.name, "Alice");
        assert_eq!(m.capabilities.len(), 2);
        assert_eq!(m.status, MemberStatus::Idle);
    }

    #[test]
    fn test_coordinator_members() {
        let team = test_team();
        assert_eq!(team.member_count(), 3);
        assert!(team.get_member("lead").is_some());
        assert!(team.get_member("unknown").is_none());
        assert!(team.lead().is_some());
        assert_eq!(team.lead().unwrap().name, "Alice");
    }

    #[test]
    fn test_coordinator_remove_member() {
        let mut team = test_team();
        assert!(team.remove_member("dev2"));
        assert_eq!(team.member_count(), 2);
        assert!(!team.remove_member("nonexistent"));
    }

    #[test]
    fn test_peer_message_text() {
        let msg = PeerMessage::text("a1", "a2", "hello");
        assert_eq!(msg.from.0, "a1");
        assert_eq!(msg.to.0, "a2");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.message_type, MessageType::Text);
        assert!(!msg.read);
    }

    #[test]
    fn test_peer_message_request() {
        let msg = PeerMessage::request("a1", "a2", "need help");
        assert_eq!(msg.message_type, MessageType::Request);
    }

    #[test]
    fn test_peer_message_response() {
        let msg = PeerMessage::response("a2", "a1", "here you go", "msg-1");
        assert_eq!(msg.message_type, MessageType::Response);
        assert_eq!(msg.reply_to.as_deref(), Some("msg-1"));
    }

    #[test]
    fn test_send_message() {
        let mut team = test_team();
        team.send_to_peer("lead", "dev1", "start task 1");
        assert_eq!(team.inbox("dev1").len(), 1);
        assert_eq!(team.inbox("lead").len(), 0);
    }

    #[test]
    fn test_broadcast() {
        let mut team = test_team();
        team.broadcast("lead", "meeting in 5");
        assert_eq!(team.inbox("dev1").len(), 1);
        assert_eq!(team.inbox("dev2").len(), 1);
        assert_eq!(team.inbox("lead").len(), 0);
    }

    #[test]
    fn test_unread_count() {
        let mut team = test_team();
        team.send_to_peer("lead", "dev1", "msg 1");
        team.send_to_peer("lead", "dev1", "msg 2");
        assert_eq!(team.unread_count("dev1"), 2);
        team.mark_read("dev1");
        assert_eq!(team.unread_count("dev1"), 0);
    }

    #[test]
    fn test_conversation() {
        let mut team = test_team();
        team.send_to_peer("lead", "dev1", "hello");
        team.send_to_peer("dev1", "lead", "hi back");
        team.send_to_peer("lead", "dev2", "other");
        let conv = team.conversation("lead", "dev1");
        assert_eq!(conv.len(), 2);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = SharedTask::new("t1", "Implement feature", "lead");
        assert_eq!(task.status, TaskStatus::Pending);
        task.assign("dev1");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.assignee.as_ref().unwrap().0, "dev1");
        task.touch_file("src/main.rs");
        assert_eq!(task.files_touched.len(), 1);
        // Touch same file again — no duplicate
        task.touch_file("src/main.rs");
        assert_eq!(task.files_touched.len(), 1);
        task.complete("done");
        assert_eq!(task.status, TaskStatus::Complete);
        assert_eq!(task.output.as_deref(), Some("done"));
    }

    #[test]
    fn test_task_block() {
        let mut task = SharedTask::new("t1", "Test", "lead");
        task.block("waiting for API");
        assert!(matches!(task.status, TaskStatus::Blocked(_)));
        assert_eq!(task.status.as_str(), "blocked");
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::InReview.as_str(), "in_review");
        assert_eq!(TaskStatus::Complete.as_str(), "complete");
        assert_eq!(TaskStatus::Failed("err".into()).as_str(), "failed");
    }

    #[test]
    fn test_coordinator_tasks() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "Task 1", "lead"));
        team.add_task(SharedTask::new("t2", "Task 2", "lead"));
        assert_eq!(team.task_count(), 2);
        assert!(team.get_task("t1").is_some());
    }

    #[test]
    fn test_assign_task() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "Task 1", "lead"));
        assert!(team.assign_task("t1", "dev1"));
        assert_eq!(team.get_task("t1").unwrap().status, TaskStatus::InProgress);
        assert!(!team.assign_task("nonexistent", "dev1"));
    }

    #[test]
    fn test_complete_task() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "Task 1", "lead"));
        assert!(team.complete_task("t1", "all done"));
        assert_eq!(team.get_task("t1").unwrap().status, TaskStatus::Complete);
    }

    #[test]
    fn test_tasks_by_status() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "Task 1", "lead"));
        team.add_task(SharedTask::new("t2", "Task 2", "lead"));
        team.assign_task("t1", "dev1");
        assert_eq!(team.tasks_by_status("in_progress").len(), 1);
        assert_eq!(team.tasks_by_status("pending").len(), 1);
    }

    #[test]
    fn test_tasks_for_agent() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "T1", "lead"));
        team.add_task(SharedTask::new("t2", "T2", "lead"));
        team.assign_task("t1", "dev1");
        team.assign_task("t2", "dev1");
        assert_eq!(team.tasks_for_agent("dev1").len(), 2);
        assert_eq!(team.tasks_for_agent("dev2").len(), 0);
    }

    #[test]
    fn test_conflict_detection() {
        let mut team = test_team();
        let mut t1 = SharedTask::new("t1", "T1", "lead");
        t1.assign("dev1");
        t1.touch_file("src/lib.rs");
        let mut t2 = SharedTask::new("t2", "T2", "lead");
        t2.assign("dev2");
        t2.touch_file("src/lib.rs");
        team.add_task(t1);
        team.add_task(t2);
        let conflicts = team.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].file, "src/lib.rs");
    }

    #[test]
    fn test_no_conflict_same_agent() {
        let mut team = test_team();
        let mut t1 = SharedTask::new("t1", "T1", "lead");
        t1.assign("dev1");
        t1.touch_file("src/lib.rs");
        let mut t2 = SharedTask::new("t2", "T2", "lead");
        t2.assign("dev1");
        t2.touch_file("src/lib.rs");
        team.add_task(t1);
        team.add_task(t2);
        let conflicts = team.detect_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_no_conflict_different_files() {
        let mut team = test_team();
        let mut t1 = SharedTask::new("t1", "T1", "lead");
        t1.assign("dev1");
        t1.touch_file("src/a.rs");
        let mut t2 = SharedTask::new("t2", "T2", "lead");
        t2.assign("dev2");
        t2.touch_file("src/b.rs");
        team.add_task(t1);
        team.add_task(t2);
        let conflicts = team.detect_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_resolve_conflict() {
        let mut team = test_team();
        let mut t1 = SharedTask::new("t1", "T1", "lead");
        t1.assign("dev1");
        t1.touch_file("file.rs");
        let mut t2 = SharedTask::new("t2", "T2", "lead");
        t2.assign("dev2");
        t2.touch_file("file.rs");
        team.add_task(t1);
        team.add_task(t2);
        team.detect_conflicts();
        assert_eq!(team.unresolved_conflicts().len(), 1);
        assert!(team.resolve_conflict("file.rs", ConflictResolution::Merge));
        assert!(team.unresolved_conflicts().is_empty());
    }

    #[test]
    fn test_synthesis() {
        let mut team = test_team();
        let mut t1 = SharedTask::new("t1", "Feature A", "lead");
        t1.touch_file("a.rs");
        t1.complete("Added feature A");
        let mut t2 = SharedTask::new("t2", "Feature B", "lead");
        t2.touch_file("b.rs");
        t2.complete("Added feature B");
        team.add_task(t1);
        team.add_task(t2);
        team.add_task(SharedTask::new("t3", "Pending", "lead"));
        let report = team.synthesize();
        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.completed_count, 2);
        assert_eq!(report.pending_count, 1);
        assert_eq!(report.files_modified.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut team = test_team();
        team.add_task(SharedTask::new("t1", "T1", "lead"));
        team.send_to_peer("lead", "dev1", "hi");
        let stats = team.stats();
        assert_eq!(stats.member_count, 3);
        assert_eq!(stats.total_messages, 1);
        assert_eq!(stats.total_tasks, 1);
    }

    #[test]
    fn test_member_status_update() {
        let mut team = test_team();
        if let Some(m) = team.get_member_mut("dev1") {
            m.status = MemberStatus::Working;
        }
        assert_eq!(team.get_member("dev1").unwrap().status, MemberStatus::Working);
    }

    #[test]
    fn test_list_members() {
        let team = test_team();
        let members = team.list_members();
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn test_conflict_resolution_variants() {
        assert_eq!(ConflictResolution::KeepA, ConflictResolution::KeepA);
        assert_ne!(ConflictResolution::KeepA, ConflictResolution::KeepB);
        assert_ne!(ConflictResolution::Merge, ConflictResolution::KeepA);
    }
}
