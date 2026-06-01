#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! Tainted boundary for RAG / semantic-index retrieval — when VibeCody
//! pulls chunks from the local embedding index and feeds them into the
//! model's context.
//!
//! DREAD #1 Slice E. Retrieval hits originate from indexed files
//! (T5 — file contents we don't author). When the agent loop quotes a
//! hit's text into the next prompt, a prompt-injection payload that
//! happens to live in a README, vendored dependency, or stale .cache
//! file is now reading instructions to the model. Slice B / C / D
//! gate the model's *outputs*; Slice E tags the model's *inputs* so
//! when those inputs round-trip into a tool-call argument, the
//! shell / http gates see `Provenance::Rag` and can attribute the
//! rejection back to a specific (index, doc_id) pair in the audit
//! log.
//!
//! ## Where this helper sits
//!
//! `vibe_core::index::embeddings::EmbeddingIndex::search` is the raw
//! retrieval API. It returns `Result<Vec<SearchHit>>` where each hit
//! carries the indexed `text: String`. That text crosses our T0/T5
//! trust boundary at the moment it leaves the retrieval call.
//!
//! Like the [`crate::mcp_taint`] boundary, `vibe-core` doesn't depend
//! on `vibecli-cli`, so the `Tainted` type can't live there without a
//! workspace-level refactor. This module is the typed boundary helper
//! for every call site in `vibecli-cli` that retrieves from the
//! semantic index.
//!
//! ## Current state
//!
//! `main.rs` has interactive `/index` / `/search` commands and a
//! prompt-context builder that quotes hits verbatim. Those call sites
//! currently consume `SearchHit.text` directly. Slice E adds the
//! helper now; migrating the existing call sites is a follow-up
//! tracked under the §8 #1 row in `threat-model.md`. The semgrep rule
//! `.semgrep/rag-taint-boundary.yml` blocks new direct consumers
//! outside this module so the migration can land at its own pace
//! without backsliding.
//!
//! See [`docs/security/tainted-data-flow.md`](../../docs/security/tainted-data-flow.md) §5
//! entry #4.

use anyhow::Result;

use crate::tainted::{Provenance, Tainted};

/// A retrieval hit with its `text` field wrapped in [`Tainted<String>`].
/// The other fields (`file`, `chunk_start`, `chunk_end`, `score`) are
/// metadata — they're not T5-controlled in any meaningful sense (the
/// path comes from our own indexer; line numbers are integers) so
/// they stay untainted for natural use in formatters / UI.
///
/// Constructed exclusively by [`search_tainted`]; the absence of a
/// `pub` constructor is part of the boundary invariant.
#[derive(Debug, Clone)]
pub struct TaintedRagHit {
    pub file: std::path::PathBuf,
    pub chunk_start: usize,
    pub chunk_end: usize,
    pub score: f32,
    /// The hit's text content, tainted with [`Provenance::Rag`].
    pub text: Tainted<String>,
}

impl TaintedRagHit {
    /// Stable identifier for audit-log correlation: `"<file>:<start>-<end>"`.
    /// Matches the `doc_id` recorded in the `Tainted<String>`'s provenance.
    pub fn doc_id(&self) -> String {
        format!(
            "{}:{}-{}",
            self.file.display(),
            self.chunk_start,
            self.chunk_end
        )
    }
}

/// Run a semantic search and wrap every hit's `text` field in
/// [`Tainted<String>`] at the boundary.
///
/// The `index_name` is recorded so admin policy can later treat
/// different indexes differently (e.g. allow hits from the user's
/// own workspace index but require confirmation for hits from a
/// vendored / git-imported index). Each hit's `doc_id` (file +
/// line range) and `score` are recorded in the provenance so the
/// confirmation modal can show the user *which document* a tainted
/// snippet came from.
///
/// Callers in the agent loop **must** use this helper rather than
/// invoking `index.search(...)` directly. The semgrep rule
/// `.semgrep/rag-taint-boundary.yml` guards the boundary.
pub async fn search_tainted(
    index: &vibe_core::index::embeddings::EmbeddingIndex,
    index_name: impl Into<String>,
    query: &str,
    k: usize,
) -> Result<Vec<TaintedRagHit>> {
    let index_name = index_name.into();

    tracing::debug!(
        target: "vibecody::tainted::rag_boundary",
        index = %index_name,
        k,
        query_bytes = query.len(),
        "rag.search dispatched (tainted boundary)",
    );

    let raw_hits = index.search(query, k).await?;

    tracing::debug!(
        target: "vibecody::tainted::rag_boundary",
        index = %index_name,
        hits = raw_hits.len(),
        "rag.search returned (wrapping each hit with Provenance::Rag)",
    );

    Ok(raw_hits
        .into_iter()
        .map(|h| {
            let doc_id = format!("{}:{}-{}", h.file.display(), h.chunk_start, h.chunk_end);
            TaintedRagHit {
                file: h.file,
                chunk_start: h.chunk_start,
                chunk_end: h.chunk_end,
                score: h.score,
                text: Tainted::new(
                    h.text,
                    Provenance::Rag {
                        index: index_name.clone(),
                        doc_id,
                        score: h.score,
                    },
                ),
            }
        })
        .collect())
}

/// Per-index admin-policy hook — symmetric with
/// [`crate::mcp_taint::audit_mcp_response`]. Today this is a no-op for
/// RAG-provenance values and rejects anything else (invariant: only
/// [`search_tainted`] constructs RAG-tainted strings). Slice G's
/// admin-policy engine plugs in here without changing the signature.
pub fn audit_rag_hit(hit: &TaintedRagHit) -> std::result::Result<(), String> {
    match hit.text.origin() {
        Provenance::Rag {
            index,
            doc_id,
            score,
        } => {
            tracing::debug!(
                target: "vibecody::tainted::rag_boundary",
                index = %index,
                doc_id = %doc_id,
                score,
                fingerprint = %hit.text.log_fingerprint(),
                "rag.hit audited (slice E policy hook — no-op until slice G ships admin policy)",
            );
            Ok(())
        }
        other => Err(format!(
            "audit_rag_hit received non-RAG provenance: {} \
             (boundary helper invariant violated)",
            other.kind()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tainted::Reason;

    fn make_hit(file: &str, start: usize, end: usize, text: &str, index: &str) -> TaintedRagHit {
        TaintedRagHit {
            file: file.into(),
            chunk_start: start,
            chunk_end: end,
            score: 0.42,
            text: Tainted::new(
                text.to_string(),
                Provenance::Rag {
                    index: index.into(),
                    doc_id: format!("{file}:{start}-{end}"),
                    score: 0.42,
                },
            ),
        }
    }

    #[test]
    fn doc_id_matches_provenance_doc_id() {
        let hit = make_hit("/repo/README.md", 10, 25, "ignore previous", "ws");
        assert_eq!(hit.doc_id(), "/repo/README.md:10-25");
        match hit.text.origin() {
            Provenance::Rag { doc_id, .. } => assert_eq!(doc_id, "/repo/README.md:10-25"),
            _ => panic!("expected RAG provenance"),
        }
    }

    #[test]
    fn tainted_hit_text_redacts_in_debug() {
        let hit = make_hit("/repo/README.md", 1, 2, "secret instruction", "ws");
        let s = format!("{:?}", hit.text);
        assert_eq!(s, "[tainted/rag]");
        assert!(!s.contains("secret"));
    }

    #[test]
    fn tainted_hit_text_exposes_under_legitimate_reason() {
        let hit = make_hit("/repo/x.md", 1, 2, "doc body", "ws");
        assert_eq!(hit.text.expose_for(Reason::LlmRequestBody), "doc body");
    }

    #[test]
    fn audit_rag_hit_accepts_rag_provenance() {
        let hit = make_hit("/repo/x.md", 1, 2, "doc", "ws");
        assert!(audit_rag_hit(&hit).is_ok());
    }

    #[test]
    fn audit_rag_hit_rejects_non_rag_provenance() {
        // Construct a TaintedRagHit whose `text` has a non-RAG origin
        // (only possible through this test's direct field access — the
        // boundary helper is the only legal constructor of RAG taint).
        let bad = TaintedRagHit {
            file: "/x".into(),
            chunk_start: 0,
            chunk_end: 1,
            score: 0.0,
            text: Tainted::from_file("/repo/x", "wrong origin".into()),
        };
        let err = audit_rag_hit(&bad).unwrap_err();
        assert!(err.contains("non-RAG provenance"), "got: {err}");
        assert!(err.contains("file"), "got: {err}");
    }
}
