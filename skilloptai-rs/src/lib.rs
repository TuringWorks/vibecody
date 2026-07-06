//! # skilloptai-rs — train agent-skill documents.
//!
//! Rust port of [TuringWorks/SkillOpt](https://github.com/TuringWorks/SkillOpt).
//! Treats a skill markdown doc as the trainable state of a frozen agent:
//! scored **rollouts** drive bounded **add/delete/replace edits**, accepted
//! **only when a held-out validation score strictly improves**, epoch after
//! epoch — with zero inference-time overhead at deploy.
//!
//! Design: `notes/skillforge/03 — skilloptai-rs — Design.md`.
//!
//! ## Layering
//! Depends on [`skilllensai`] for the shared `Trajectory` schema, the
//! `SkillLlm` seam, and the `metrics` used by the validation gate
//! (`target_evolvability`). Depends on neither `vibecli` nor `vibe-ai`.
//!
//! ## Feature map
//! - `sqlite` (default) — trajectory/run persistence.
//! - `llm` (default) — the async training loop over the `SkillLlm` seam.
//! - `cli` — the `skilloptai` binary.

pub mod buffer;
pub mod edit;
pub mod report;

#[cfg(feature = "llm")]
pub mod env;
#[cfg(feature = "llm")]
pub mod gate;
#[cfg(feature = "llm")]
pub mod propose;
#[cfg(feature = "llm")]
pub mod rollout;
#[cfg(feature = "llm")]
pub mod trainer;

pub use buffer::{canonical_key, RejectReason, RejectedEditBuffer, RejectedEntry};
pub use edit::{within_budget, EditOp};
pub use report::{approx_tokens, render_skill, TrainingReport};

#[cfg(feature = "llm")]
pub use env::{Env, StaticEnv, TaskSpec};
#[cfg(feature = "llm")]
pub use trainer::{train, TrainConfig};

/// Crate version — surfaced in the daemon `/health` block and startup banner.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
