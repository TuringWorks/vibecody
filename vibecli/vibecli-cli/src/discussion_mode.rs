
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscussionMode {
    Brainstorm,
    Review,
    DesignCritique,
    TechDecision,
    ArchitectureReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Question,
    Answer,
    Suggestion,
    Concern,
    Decision,
    Action,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Reaction {
    Agree,
    Disagree,
    Interesting,
    NeedsMoreInfo,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildState {
    Building,
    Discussing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub author: String,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub reactions: Vec<Reaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionSession {
    pub id: String,
    pub topic: String,
    pub messages: Vec<Message>,
    pub mode: DiscussionMode,
    pub participants: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionSummary {
    pub topic: String,
    pub decision_count: usize,
    pub action_count: usize,
    pub unresolved_count: usize,
    pub key_decisions: Vec<String>,
    pub key_actions: Vec<String>,
}

pub struct DiscussionManager {
    sessions: HashMap<String, DiscussionSession>,
    build_state: BuildState,
    next_session_id: u64,
    next_message_id: u64,
}

impl DiscussionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            build_state: BuildState::Building,
            next_session_id: 1,
            next_message_id: 1,
        }
    }

    pub fn start_discussion(&mut self, topic: &str, mode: DiscussionMode) -> DiscussionSession {
        let id = format!("session-{}", self.next_session_id);
        self.next_session_id += 1;

        let session = DiscussionSession {
            id: id.clone(),
            topic: topic.to_string(),
            messages: Vec::new(),
            mode,
            participants: Vec::new(),
            created_at: self.current_timestamp(),
        };

        self.sessions.insert(id, session.clone());
        session
    }

    pub fn add_message(
        &mut self,
        session_id: &str,
        author: &str,
        content: &str,
        message_type: MessageType,
    ) -> Option<Message> {
        let msg_id = format!("msg-{}", self.next_message_id);
        self.next_message_id += 1;

        let message = Message {
            id: msg_id,
            author: author.to_string(),
            content: content.to_string(),
            message_type,
            timestamp: self.current_timestamp(),
            reactions: Vec::new(),
        };

        let session = self.sessions.get_mut(session_id)?;

        if !session.participants.contains(&author.to_string()) {
            session.participants.push(author.to_string());
        }

        session.messages.push(message.clone());
        Some(message)
    }

    pub fn add_reaction(
        &mut self,
        session_id: &str,
        message_id: &str,
        reaction: Reaction,
    ) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            if let Some(msg) = session.messages.iter_mut().find(|m| m.id == message_id) {
                msg.reactions.push(reaction);
                return true;
            }
        }
        false
    }

    pub fn get_decisions(&self, session_id: &str) -> Vec<Message> {
        self.filter_messages(session_id, |m| m.message_type == MessageType::Decision)
    }

    pub fn get_action_items(&self, session_id: &str) -> Vec<Message> {
        self.filter_messages(session_id, |m| m.message_type == MessageType::Action)
    }

    pub fn get_unresolved(&self, session_id: &str) -> Vec<Message> {
        self.filter_messages(session_id, |m| {
            m.message_type == MessageType::Concern
                && !m.reactions.contains(&Reaction::Resolved)
        })
    }

    pub fn summarize(&self, session_id: &str) -> Option<DiscussionSummary> {
        let session = self.sessions.get(session_id)?;

        let decisions = self.get_decisions(session_id);
        let actions = self.get_action_items(session_id);
        let unresolved = self.get_unresolved(session_id);

        Some(DiscussionSummary {
            topic: session.topic.clone(),
            decision_count: decisions.len(),
            action_count: actions.len(),
            unresolved_count: unresolved.len(),
            key_decisions: decisions.iter().map(|m| m.content.clone()).collect(),
            key_actions: actions.iter().map(|m| m.content.clone()).collect(),
        })
    }

    pub fn pause_build(&mut self) {
        self.build_state = BuildState::Paused;
    }

    pub fn resume_build(&mut self) {
        self.build_state = BuildState::Building;
    }

    pub fn build_state(&self) -> &BuildState {
        &self.build_state
    }

    pub fn set_discussing(&mut self) {
        self.build_state = BuildState::Discussing;
    }

    pub fn list_sessions(&self) -> Vec<DiscussionSession> {
        self.sessions.values().cloned().collect()
    }

    fn filter_messages<F>(&self, session_id: &str, predicate: F) -> Vec<Message>
    where
        F: Fn(&Message) -> bool,
    {
        self.sessions
            .get(session_id)
            .map(|s| s.messages.iter().filter(|m| predicate(m)).cloned().collect())
            .unwrap_or_default()
    }

    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with_session() -> (DiscussionManager, String) {
        let mut mgr = DiscussionManager::new();
        let session = mgr.start_discussion("API design", DiscussionMode::Brainstorm);
        (mgr, session.id)
    }

    #[test]
    fn test_start_discussion() {
        let mut mgr = DiscussionManager::new();
        let session = mgr.start_discussion("Topic A", DiscussionMode::Review);
        assert_eq!(session.topic, "Topic A");
        assert_eq!(session.mode, DiscussionMode::Review);
        assert!(session.messages.is_empty());
        assert!(session.participants.is_empty());
    }

    #[test]
    fn test_session_ids_increment() {
        let mut mgr = DiscussionManager::new();
        let s1 = mgr.start_discussion("A", DiscussionMode::Brainstorm);
        let s2 = mgr.start_discussion("B", DiscussionMode::Review);
        assert_eq!(s1.id, "session-1");
        assert_eq!(s2.id, "session-2");
    }

    #[test]
    fn test_add_message() {
        let (mut mgr, sid) = manager_with_session();
        let msg = mgr.add_message(&sid, "alice", "What about REST?", MessageType::Question);
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.author, "alice");
        assert_eq!(msg.message_type, MessageType::Question);
    }

    #[test]
    fn test_add_message_invalid_session() {
        let mut mgr = DiscussionManager::new();
        let result = mgr.add_message("nonexistent", "alice", "hi", MessageType::Note);
        assert!(result.is_none());
    }

    #[test]
    fn test_participants_tracked() {
        let (mut mgr, sid) = manager_with_session();
        mgr.add_message(&sid, "alice", "msg1", MessageType::Note);
        mgr.add_message(&sid, "bob", "msg2", MessageType::Note);
        mgr.add_message(&sid, "alice", "msg3", MessageType::Note);

        let session = &mgr.sessions[&sid];
        assert_eq!(session.participants.len(), 2);
        assert!(session.participants.contains(&"alice".to_string()));
        assert!(session.participants.contains(&"bob".to_string()));
    }

    #[test]
    fn test_add_reaction() {
        let (mut mgr, sid) = manager_with_session();
        let msg = mgr.add_message(&sid, "alice", "idea", MessageType::Suggestion).unwrap();
        let ok = mgr.add_reaction(&sid, &msg.id, Reaction::Agree);
        assert!(ok);

        let session = &mgr.sessions[&sid];
        assert_eq!(session.messages[0].reactions.len(), 1);
        assert_eq!(session.messages[0].reactions[0], Reaction::Agree);
    }

    #[test]
    fn test_add_reaction_invalid_message() {
        let (mut mgr, sid) = manager_with_session();
        let ok = mgr.add_reaction(&sid, "bogus", Reaction::Agree);
        assert!(!ok);
    }

    #[test]
    fn test_add_reaction_invalid_session() {
        let mut mgr = DiscussionManager::new();
        let ok = mgr.add_reaction("bogus", "msg-1", Reaction::Agree);
        assert!(!ok);
    }

    #[test]
    fn test_get_decisions() {
        let (mut mgr, sid) = manager_with_session();
        mgr.add_message(&sid, "alice", "Use GraphQL", MessageType::Decision);
        mgr.add_message(&sid, "bob", "What about REST?", MessageType::Question);
        mgr.add_message(&sid, "alice", "Add pagination", MessageType::Decision);

        let decisions = mgr.get_decisions(&sid);
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].content, "Use GraphQL");
        assert_eq!(decisions[1].content, "Add pagination");
    }

    #[test]
    fn test_get_action_items() {
        let (mut mgr, sid) = manager_with_session();
        mgr.add_message(&sid, "alice", "Write RFC", MessageType::Action);
        mgr.add_message(&sid, "bob", "Sounds good", MessageType::Note);
        mgr.add_message(&sid, "bob", "Benchmark options", MessageType::Action);

        let actions = mgr.get_action_items(&sid);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].content, "Write RFC");
    }

    #[test]
    fn test_get_unresolved_concerns() {
        let (mut mgr, sid) = manager_with_session();
        let c1 = mgr.add_message(&sid, "alice", "Perf risk", MessageType::Concern).unwrap();
        mgr.add_message(&sid, "bob", "Security gap", MessageType::Concern);

        // Resolve the first concern
        mgr.add_reaction(&sid, &c1.id, Reaction::Resolved);

        let unresolved = mgr.get_unresolved(&sid);
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].content, "Security gap");
    }

    #[test]
    fn test_all_concerns_resolved() {
        let (mut mgr, sid) = manager_with_session();
        let c1 = mgr.add_message(&sid, "alice", "Issue A", MessageType::Concern).unwrap();
        mgr.add_reaction(&sid, &c1.id, Reaction::Resolved);

        let unresolved = mgr.get_unresolved(&sid);
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_summarize() {
        let (mut mgr, sid) = manager_with_session();
        mgr.add_message(&sid, "alice", "Use Rust", MessageType::Decision);
        mgr.add_message(&sid, "bob", "Write tests", MessageType::Action);
        mgr.add_message(&sid, "alice", "Memory safety?", MessageType::Concern);

        let summary = mgr.summarize(&sid).unwrap();
        assert_eq!(summary.topic, "API design");
        assert_eq!(summary.decision_count, 1);
        assert_eq!(summary.action_count, 1);
        assert_eq!(summary.unresolved_count, 1);
        assert_eq!(summary.key_decisions, vec!["Use Rust"]);
        assert_eq!(summary.key_actions, vec!["Write tests"]);
    }

    #[test]
    fn test_summarize_invalid_session() {
        let mgr = DiscussionManager::new();
        assert!(mgr.summarize("bogus").is_none());
    }

    #[test]
    fn test_build_state_default() {
        let mgr = DiscussionManager::new();
        assert_eq!(*mgr.build_state(), BuildState::Building);
    }

    #[test]
    fn test_pause_build() {
        let mut mgr = DiscussionManager::new();
        mgr.pause_build();
        assert_eq!(*mgr.build_state(), BuildState::Paused);
    }

    #[test]
    fn test_resume_build() {
        let mut mgr = DiscussionManager::new();
        mgr.pause_build();
        mgr.resume_build();
        assert_eq!(*mgr.build_state(), BuildState::Building);
    }

    #[test]
    fn test_set_discussing() {
        let mut mgr = DiscussionManager::new();
        mgr.set_discussing();
        assert_eq!(*mgr.build_state(), BuildState::Discussing);
    }

    #[test]
    fn test_build_state_transitions() {
        let mut mgr = DiscussionManager::new();
        assert_eq!(*mgr.build_state(), BuildState::Building);
        mgr.set_discussing();
        assert_eq!(*mgr.build_state(), BuildState::Discussing);
        mgr.pause_build();
        assert_eq!(*mgr.build_state(), BuildState::Paused);
        mgr.resume_build();
        assert_eq!(*mgr.build_state(), BuildState::Building);
    }

    #[test]
    fn test_list_sessions_empty() {
        let mgr = DiscussionManager::new();
        assert!(mgr.list_sessions().is_empty());
    }

    #[test]
    fn test_list_sessions_multiple() {
        let mut mgr = DiscussionManager::new();
        mgr.start_discussion("A", DiscussionMode::Brainstorm);
        mgr.start_discussion("B", DiscussionMode::TechDecision);
        mgr.start_discussion("C", DiscussionMode::ArchitectureReview);

        let sessions = mgr.list_sessions();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_discussion_modes() {
        let mut mgr = DiscussionManager::new();
        let s1 = mgr.start_discussion("a", DiscussionMode::DesignCritique);
        let s2 = mgr.start_discussion("b", DiscussionMode::TechDecision);
        let s3 = mgr.start_discussion("c", DiscussionMode::ArchitectureReview);
        assert_eq!(s1.mode, DiscussionMode::DesignCritique);
        assert_eq!(s2.mode, DiscussionMode::TechDecision);
        assert_eq!(s3.mode, DiscussionMode::ArchitectureReview);
    }

    #[test]
    fn test_multiple_reactions_on_message() {
        let (mut mgr, sid) = manager_with_session();
        let msg = mgr.add_message(&sid, "alice", "idea", MessageType::Suggestion).unwrap();
        mgr.add_reaction(&sid, &msg.id, Reaction::Agree);
        mgr.add_reaction(&sid, &msg.id, Reaction::Interesting);
        mgr.add_reaction(&sid, &msg.id, Reaction::NeedsMoreInfo);

        let session = &mgr.sessions[&sid];
        assert_eq!(session.messages[0].reactions.len(), 3);
    }

    #[test]
    fn test_message_types_filter_correctly() {
        let (mut mgr, sid) = manager_with_session();
        mgr.add_message(&sid, "a", "q1", MessageType::Question);
        mgr.add_message(&sid, "a", "a1", MessageType::Answer);
        mgr.add_message(&sid, "a", "s1", MessageType::Suggestion);
        mgr.add_message(&sid, "a", "c1", MessageType::Concern);
        mgr.add_message(&sid, "a", "d1", MessageType::Decision);
        mgr.add_message(&sid, "a", "act1", MessageType::Action);
        mgr.add_message(&sid, "a", "n1", MessageType::Note);

        assert_eq!(mgr.get_decisions(&sid).len(), 1);
        assert_eq!(mgr.get_action_items(&sid).len(), 1);
        assert_eq!(mgr.get_unresolved(&sid).len(), 1);
    }

    #[test]
    fn test_get_decisions_empty_session() {
        let (mgr, sid) = manager_with_session();
        assert!(mgr.get_decisions(&sid).is_empty());
    }

    #[test]
    fn test_get_decisions_nonexistent_session() {
        let mgr = DiscussionManager::new();
        assert!(mgr.get_decisions("nope").is_empty());
    }

    #[test]
    fn test_summary_empty_session() {
        let (mgr, sid) = manager_with_session();
        let summary = mgr.summarize(&sid).unwrap();
        assert_eq!(summary.decision_count, 0);
        assert_eq!(summary.action_count, 0);
        assert_eq!(summary.unresolved_count, 0);
        assert!(summary.key_decisions.is_empty());
        assert!(summary.key_actions.is_empty());
    }
}
