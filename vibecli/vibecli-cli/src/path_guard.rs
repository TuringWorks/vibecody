//! Path-traversal gate — thin re-export over `vibe_core::path_guard`.
//!
//! See [`vibe_core::path_guard`] for the canonical implementation.
//! This shim preserves the `crate::path_guard::reject_sensitive_path`
//! call-site path used by `serve.rs`, `watch_bridge.rs`, and other
//! daemon-side consumers so a future module move doesn't ripple
//! through every caller.
//!
//! The promotion was done in DREAD #2 cleanup
//! (`docs/security/threat-model.md` §10 entry for 2026-05-15) to
//! collapse what had become four near-identical copies of the gate
//! into one source of truth.

pub use vibe_core::path_guard::{
    canonicalize_lenient,
    reject_sensitive_path,
    DENIED_FILENAMES,
    DENIED_SEGMENTS,
};
