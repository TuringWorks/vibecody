//! Graph analytics — communities, god nodes, blast radius.
//!
//! Borrows ideas from Graphify: community detection (topology-only, no embeddings),
//! "god node" identification (highest-degree keystones), and "surprising" cross-file
//! edges. The [`blast_radius`] module is the token-reduction primitive consumers wire
//! into context selection.

pub mod blast_radius;
pub mod communities;
pub mod god_nodes;

pub use blast_radius::{blast_radius, BlastRadius};
pub use communities::{detect_communities, Community, CommunityDetector};
pub use god_nodes::{god_nodes, surprising_edges, GodNode, SurprisingEdge};