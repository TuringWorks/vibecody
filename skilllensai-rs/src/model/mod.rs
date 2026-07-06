//! Core data model.
//!
//! [`trajectory`] holds the **unified Trajectory schema** — the shared plumbing
//! both `skilllensai-rs` and `skilloptai-rs` speak. [`skill`] parses a
//! VibeCody `skills/*.md` file; [`experience`] pools trajectories before
//! extraction.

pub mod experience;
pub mod skill;
pub mod trajectory;

pub use experience::ExperiencePool;
pub use skill::Skill;
pub use trajectory::{Outcome, Role, Step, ToolCall, Trajectory};
