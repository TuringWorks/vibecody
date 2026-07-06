//! # skillforgeai-rs — the SkillForge facade.
//!
//! Convenience umbrella that re-exports the two SkillForge crates so downstream
//! code can depend on one name:
//!
//! - [`lens`] — [`skilllensai`]: analyse & measure skill utility.
//! - [`opt`] — [`skilloptai`]: train skill documents.
//!
//! They compose: **lens picks/measures → opt optimises → lens re-measures.**
//! See `notes/skillforge/` (start at `SkillForge — MOC.md`).

/// The analysis crate — trajectory normalisation, extraction, metrics.
pub use skilllensai as lens;

/// The optimizer crate — the rollout → edit → validation-gate → epoch loop.
pub use skilloptai as opt;

/// Facade version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
