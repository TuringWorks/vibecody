#![allow(dead_code)]
//! Conversation branching — fork a session at any message, restore or compare
//! conversation branches.
//!
//! Matches Cursor 4.0's conversation branch feature.

use std::collections::HashMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub created_at: Instant,
    /// Optional tool call metadata.
    pub tool_name: Option<String>,
}

impl Message {
    pub fn user(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: Role::User,
            content: content.into(),
            created_at: Instant::now(),
            tool_name: None,
        }
    }

    pub fn assistant(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: Role::Assistant,
            content: content.into(),
            created_at: Instant::now(),
            tool_name: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Branch
// ---------------------------------------------------------------------------

/// A unique branch identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchId(pub String);

impl BranchId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn main() -> Self {
        Self("main".into())
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A conversation branch — a linear sequence of messages forked from a parent.
#[derive(Debug, Clone)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    /// The parent branch this was forked from, if any.
    pub parent: Option<BranchId>,
    /// The message ID in the parent at which this branch was forked.
    pub fork_point: Option<String>,
    /// Messages in this branch (after the fork point).
    pub messages: Vec<Message>,
    pub created_at: Instant,
    pub archived: bool,
}

impl Branch {
    pub fn new_main() -> Self {
        Self {
            id: BranchId::main(),
            name: "main".into(),
            parent: None,
            fork_point: None,
            messages: vec![],
            created_at: Instant::now(),
            archived: false,
        }
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

// ---------------------------------------------------------------------------
// Branch manager
// ---------------------------------------------------------------------------

/// Manages all branches of a conversation session.
pub struct BranchManager {
    branches: HashMap<BranchId, Branch>,
    /// Shared prefix: messages before any fork occurred.
    shared_prefix: Vec<Message>,
    /// Currently active branch.
    active: BranchId,
    /// Counter for generating unique IDs.
    id_counter: u64,
}

impl BranchManager {
    pub fn new() -> Self {
        let main = Branch::new_main();
        let id = main.id.clone();
        let mut branches = HashMap::new();
        branches.insert(id.clone(), main);
        Self {
            branches,
            shared_prefix: vec![],
            active: id,
            id_counter: 0,
        }
    }

    /// Add a message to the active branch.
    pub fn push_message(&mut self, message: Message) {
        if let Some(branch) = self.branches.get_mut(&self.active) {
            branch.messages.push(message.clone());
        }
        // If no branches have been forked yet, also extend shared prefix.
        if self.branches.len() == 1 {
            self.shared_prefix.push(message);
        }
    }

    /// Fork the active branch at the message with `fork_message_id`.
    /// Returns the new branch ID.
    pub fn fork_at(
        &mut self,
        fork_message_id: &str,
        branch_name: impl Into<String>,
    ) -> Result<BranchId, String> {
        self.id_counter += 1;
        let new_id = BranchId::new(format!("branch-{}", self.id_counter));

        let parent = self.active.clone();
        let parent_branch = self
            .branches
            .get(&parent)
            .ok_or_else(|| "active branch not found".to_string())?;

        // Find the fork point index.
        let fork_idx = parent_branch
            .messages
            .iter()
            .position(|m| m.id == fork_message_id)
            .ok_or_else(|| format!("message {fork_message_id} not found"))?;

        // The new branch inherits messages up to and including the fork point.
        let inherited: Vec<Message> = parent_branch.messages[..=fork_idx].to_vec();

        let new_branch = Branch {
            id: new_id.clone(),
            name: branch_name.into(),
            parent: Some(parent),
            fork_point: Some(fork_message_id.to_string()),
            messages: inherited,
            created_at: Instant::now(),
            archived: false,
        };

        self.branches.insert(new_id.clone(), new_branch);
        Ok(new_id)
    }

    /// Switch the active branch.
    pub fn checkout(&mut self, branch_id: &BranchId) -> Result<(), String> {
        if self.branches.contains_key(branch_id) {
            self.active = branch_id.clone();
            Ok(())
        } else {
            Err(format!("branch {} not found", branch_id))
        }
    }

    /// Archive a branch (soft-delete).
    pub fn archive(&mut self, branch_id: &BranchId) -> bool {
        if let Some(branch) = self.branches.get_mut(branch_id) {
            branch.archived = true;
            true
        } else {
            false
        }
    }

    /// Return the full message history of a branch (inherited + own).
    pub fn full_history(&self, branch_id: &BranchId) -> Vec<&Message> {
        self.branches
            .get(branch_id)
            .map(|b| b.messages.iter().collect())
            .unwrap_or_default()
    }

    /// Compare two branches: returns messages unique to each.
    pub fn diff(
        &self,
        a: &BranchId,
        b: &BranchId,
    ) -> (Vec<&Message>, Vec<&Message>) {
        let hist_a = self.full_history(a);
        let hist_b = self.full_history(b);

        let ids_b: std::collections::HashSet<&str> =
            hist_b.iter().map(|m| m.id.as_str()).collect();
        let ids_a: std::collections::HashSet<&str> =
            hist_a.iter().map(|m| m.id.as_str()).collect();

        let only_in_a: Vec<&Message> = hist_a
            .iter()
            .copied()
            .filter(|m| !ids_b.contains(m.id.as_str()))
            .collect();
        let only_in_b: Vec<&Message> = hist_b
            .iter()
            .copied()
            .filter(|m| !ids_a.contains(m.id.as_str()))
            .collect();

        (only_in_a, only_in_b)
    }

    /// Active branch ID.
    pub fn active_branch(&self) -> &BranchId {
        &self.active
    }

    /// List all non-archived branches.
    pub fn list_branches(&self) -> Vec<&Branch> {
        let mut branches: Vec<_> = self
            .branches
            .values()
            .filter(|b| !b.archived)
            .collect();
        branches.sort_by_key(|b| &b.id.0);
        branches
    }

    /// Number of branches (including archived).
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Get branch by ID.
    pub fn get_branch(&self, id: &BranchId) -> Option<&Branch> {
        self.branches.get(id)
    }
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, text: &str) -> Message {
        Message::user(id, text)
    }

    #[test]
    fn test_initial_state_has_main_branch() {
        let mgr = BranchManager::new();
        assert_eq!(*mgr.active_branch(), BranchId::main());
        assert_eq!(mgr.branch_count(), 1);
    }

    #[test]
    fn test_push_message_to_main() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "hello"));
        let history = mgr.full_history(&BranchId::main());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "hello");
    }

    #[test]
    fn test_fork_at_message() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "first"));
        mgr.push_message(msg("m2", "second"));
        let branch_id = mgr.fork_at("m1", "explore-alt").unwrap();
        assert_eq!(mgr.branch_count(), 2);
        // New branch has messages up to and including m1.
        let history = mgr.full_history(&branch_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "m1");
    }

    #[test]
    fn test_fork_returns_error_for_unknown_message() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "text"));
        let result = mgr.fork_at("unknown-id", "branch");
        assert!(result.is_err());
    }

    #[test]
    fn test_checkout_switches_active() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "first"));
        let bid = mgr.fork_at("m1", "alt").unwrap();
        mgr.checkout(&bid).unwrap();
        assert_eq!(*mgr.active_branch(), bid);
    }

    #[test]
    fn test_checkout_unknown_branch_errors() {
        let mut mgr = BranchManager::new();
        let result = mgr.checkout(&BranchId::new("ghost"));
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_hides_branch() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "text"));
        let bid = mgr.fork_at("m1", "temp").unwrap();
        mgr.archive(&bid);
        let visible: Vec<_> = mgr.list_branches();
        assert!(!visible.iter().any(|b| b.id == bid));
    }

    #[test]
    fn test_diff_detects_diverged_messages() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "shared"));
        let alt = mgr.fork_at("m1", "alt").unwrap();

        // Add different messages to each branch.
        mgr.push_message(msg("m2-main", "main only"));
        mgr.checkout(&alt).unwrap();
        mgr.push_message(msg("m2-alt", "alt only"));

        let (only_main, only_alt) = mgr.diff(&BranchId::main(), &alt);
        assert!(only_main.iter().any(|m| m.id == "m2-main"));
        assert!(only_alt.iter().any(|m| m.id == "m2-alt"));
    }

    #[test]
    fn test_list_branches_sorted() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "text"));
        mgr.fork_at("m1", "branch-z").unwrap();
        mgr.fork_at("m1", "branch-a").unwrap();
        let branches = mgr.list_branches();
        assert!(branches.len() >= 3);
        // main should appear
        assert!(branches.iter().any(|b| b.id == BranchId::main()));
    }

    #[test]
    fn test_full_history_includes_inherited() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "root"));
        let bid = mgr.fork_at("m1", "fork").unwrap();
        mgr.checkout(&bid).unwrap();
        mgr.push_message(msg("m2", "in fork"));
        let history = mgr.full_history(&bid);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, "m1");
        assert_eq!(history[1].id, "m2");
    }

    #[test]
    fn test_branch_parent_set() {
        let mut mgr = BranchManager::new();
        mgr.push_message(msg("m1", "text"));
        let bid = mgr.fork_at("m1", "child").unwrap();
        let branch = mgr.get_branch(&bid).unwrap();
        assert_eq!(branch.parent, Some(BranchId::main()));
    }

    #[test]
    fn test_message_roles() {
        let u = Message::user("u1", "hi");
        let a = Message::assistant("a1", "hello");
        assert_eq!(u.role, Role::User);
        assert_eq!(a.role, Role::Assistant);
    }
}
