//! D1.1 — Diffcomplete chain types.
//!
//! A "chain" is a sequence of (instruction, refinement, diff, applied?)
//! tuples produced by repeated regenerations on a single
//! (file, selection) target inside the diffcomplete modal. Chains are
//! the backbone of the recap-and-resume design for the diffcomplete
//! workstream (see `docs/design/recap-resume/03-diffcomplete.md`).
//!
//! ## Patent-distance posture
//!
//! These types are pure data; they encode *what already happened*. They
//! never run on a timer, never decorate the editor buffer, never
//! contain accept/reject affordances, and never expand a model's
//! context window. Persistence happens only on discrete user-driven
//! events (regenerate succeeded, modal closed, apply clicked).
//!
//! Re-audit (per `notes/PATENT_AUDIT_INLINE.md`):
//! Patent re-audit: PASS (elements 1–5 unchanged).
//!
//! ## Wire shape
//!
//! Mirrors the design doc exactly. snake_case JSON; numeric
//! timestamps in ISO-8601 to match the cross-cutting `Recap` shape.
//!
//! ## What this module is NOT
//!
//! No SQL — that lives in `diff_chain_store.rs`. No HTTP — that
//! lives in `serve.rs`. No editor / IDE behaviour — that lives in
//! `vibeui/src/components/DiffCompleteModal.tsx` and is out of
//! scope for D1.1 (autosave RPC only).

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stable wire shape for a diffcomplete chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffChain {
    pub id: String,
    pub workspace: PathBuf,
    pub file_path: String,
    pub language: String,
    pub selection_start: u32,
    pub selection_end: u32,
    pub original_text: String,
    pub steps: Vec<DiffChainStep>,
    pub final_state: DiffChainFinal,
    /// When set, this chain was forked from `parent_chain_id`. Forking
    /// is the only way an existing chain's prefix is reused for new
    /// steps; chain rewriting is forbidden by design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_chain_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: u16,
}

/// One regeneration in the chain. The first step (`index = 0`) carries
/// the original instruction; subsequent steps carry a refinement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffChainStep {
    pub index: u32,
    pub instruction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refinement: Option<String>,
    #[serde(default)]
    pub additional_files: Vec<AdditionalFile>,
    pub diff: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub tokens_input: u32,
    #[serde(default)]
    pub tokens_output: u32,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdditionalFile {
    pub path: String,
    pub language: String,
    /// Optional truncated content snippet the user attached as
    /// additional context. Persisted verbatim so resume can replay.
    pub content_excerpt: String,
}

/// Where the chain landed at last write. `Open` means the modal is
/// still alive; the autosave column-write is the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiffChainFinal {
    Applied {
        applied_step: u32,
        applied_at: DateTime<Utc>,
    },
    Cancelled {
        reason: CancellationReason,
        cancelled_at: DateTime<Utc>,
    },
    Open,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    UserCancel,
    ModalClosed,
    Error,
}

impl DiffChainFinal {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            DiffChainFinal::Applied { .. } => "applied",
            DiffChainFinal::Cancelled { .. } => "cancelled",
            DiffChainFinal::Open => "open",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_state_db_str_matches_design_doc() {
        assert_eq!(DiffChainFinal::Open.as_db_str(), "open");
        assert_eq!(
            DiffChainFinal::Applied {
                applied_step: 2,
                applied_at: Utc::now()
            }
            .as_db_str(),
            "applied",
        );
        assert_eq!(
            DiffChainFinal::Cancelled {
                reason: CancellationReason::UserCancel,
                cancelled_at: Utc::now()
            }
            .as_db_str(),
            "cancelled",
        );
    }

    #[test]
    fn diff_chain_step_round_trips_through_json() {
        let step = DiffChainStep {
            index: 1,
            instruction: "rename x to count".into(),
            refinement: Some("tighten the error path".into()),
            additional_files: vec![AdditionalFile {
                path: "src/types.rs".into(),
                language: "rust".into(),
                content_excerpt: "pub struct Foo;".into(),
            }],
            diff: "--- a/...\n+++ b/...\n@@ -1 +1 @@\n-x\n+count".into(),
            provider: "anthropic".into(),
            model: "claude-opus-4-7".into(),
            tokens_input: 1200,
            tokens_output: 240,
            generated_at: Utc::now(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: DiffChainStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back, step);
    }

    #[test]
    fn diff_chain_default_serialisation_omits_optional_fields() {
        let chain = DiffChain {
            id: "01HCH".into(),
            workspace: PathBuf::from("/tmp/ws"),
            file_path: "src/auth.rs".into(),
            language: "rust".into(),
            selection_start: 12,
            selection_end: 28,
            original_text: "fn validate() {}".into(),
            steps: vec![],
            final_state: DiffChainFinal::Open,
            parent_chain_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            schema_version: 1,
        };
        let v = serde_json::to_value(&chain).unwrap();
        // `parent_chain_id` skipped when None per the wire contract.
        assert!(v.get("parent_chain_id").is_none());
        assert_eq!(v["final_state"]["type"], "open");
    }
}
