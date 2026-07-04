//! Hyperedges — relationships connecting 3+ nodes that pairwise edges can't express.
//!
//! Borrowed from Graphify. Examples: all implementations of a shared protocol, all
//! functions participating in an auth flow, all concepts from one paper section. A
//! hyperedge is stored as a named group of node keys plus a kind.

use serde::{Deserialize, Serialize};

/// Kind of hyperedge group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HyperedgeKind {
    /// All implementations of a trait / interface.
    ImplementsGroup,
    /// All functions participating in a logical flow (e.g. an auth flow).
    Flow,
    /// All members of a detected community / cluster.
    Community,
    /// All symbols declared in a single module/file.
    Module,
    /// User-defined group.
    Custom,
}

/// A hyperedge: a named group of `>= 2` node keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hyperedge {
    /// Human-readable group label.
    pub label: String,
    /// Kind of group.
    pub kind: HyperedgeKind,
    /// Node keys (`Symbol::node_key` or coarse node ids) participating in the group.
    pub members: Vec<String>,
}

impl Hyperedge {
    /// Construct a hyperedge, rejecting groups with fewer than 2 members (a 1-member
    /// "hyperedge" is just a node).
    pub fn new(label: impl Into<String>, kind: HyperedgeKind, members: Vec<String>) -> Self {
        Self { label: label.into(), kind, members }
    }

    /// True if this hyperedge has at least 3 members (the "hyper" threshold).
    pub fn is_hyper(&self) -> bool {
        self.members.len() >= 3
    }
}