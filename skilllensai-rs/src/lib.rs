//! # skilllensai-rs — analyse & measure agent-skill utility.
//!
//! Rust port of [TuringWorks/SkillLens](https://github.com/TuringWorks/SkillLens).
//! Studies the skill lifecycle **experience → extraction → consumption** and
//! measures how useful a skill is to a target model (Extraction Efficacy,
//! Target Evolvability).
//!
//! Design: `notes/skillforge/02 — skilllensai-rs — Design.md`.
//!
//! ## Layering
//! This crate depends on neither `vibecli` nor `vibe-ai`. LLM access goes
//! through the crate-local [`llm::SkillLlm`] seam (STRICT provider-agnostic);
//! the vibecli bridge adapts it over `vibe_ai::AIProvider`.
//!
//! ## Feature map
//! - `sqlite` (default) — [`store`] trajectory persistence.
//! - `llm` — the [`llm::SkillLlm`] trait + LLM-backed extraction/metrics.
//! - `cli` — the `skilllensai` binary.

pub mod convert;
pub mod extract;
pub mod metrics;
pub mod model;
pub mod report;

#[cfg(feature = "llm")]
pub mod llm;

#[cfg(feature = "sqlite")]
pub mod store;

pub use model::{ExperiencePool, Outcome, Role, Skill, Step, ToolCall, Trajectory};
pub use report::SkillReport;

/// Crate version — surfaced in the daemon `/health` block and startup banner.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
