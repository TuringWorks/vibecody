//! Audit aggregator — summary primitive consumed by the recap subsystem
//! (see `docs/design/recap-resume/02-job.md`).
//!
//! Reduces a stream of `AuditEvent`s to the structured shape the recap
//! renders: totals, per-outcome counts, per-host counts, per-inject-type
//! counts, byte totals.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::audit::{AuditEvent, EgressOutcome};

/// Aggregated view of a slice of audit events. Keys are sorted (BTreeMap)
/// so serialized output is deterministic — recap rendering wants stable
/// order for "egress summary" lines.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditSummary {
    pub total_requests: u64,
    pub by_outcome: BTreeMap<String, u64>,
    pub by_host: BTreeMap<String, u64>,
    pub by_inject: BTreeMap<String, u64>,
    pub bytes_request_total: u64,
    pub bytes_response_total: u64,
}

impl AuditSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold an event into the summary. Idempotent for any individual
    /// event but not idempotent overall — call once per event.
    pub fn ingest(&mut self, e: &AuditEvent) {
        self.total_requests += 1;
        *self
            .by_outcome
            .entry(outcome_key(&e.outcome).to_string())
            .or_insert(0) += 1;
        if !e.host.is_empty() {
            *self.by_host.entry(e.host.clone()).or_insert(0) += 1;
        }
        if !e.inject.is_empty() && e.inject != "None" {
            *self.by_inject.entry(e.inject.clone()).or_insert(0) += 1;
        }
        self.bytes_request_total += e.bytes_request;
        self.bytes_response_total += e.bytes_response;
    }

    /// Build a summary from a slice of events.
    pub fn from_events(events: &[AuditEvent]) -> Self {
        let mut s = Self::new();
        for e in events {
            s.ingest(e);
        }
        s
    }

    /// Read a JSONL audit file (one AuditEvent per line) and summarize.
    /// Lines that fail to parse are skipped with a tracing warning —
    /// summary should never panic on a corrupted log.
    pub fn from_jsonl_file(path: &Path) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let mut s = Self::new();
        for (i, line) in BufReader::new(f).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(e) => s.ingest(&e),
                Err(e) => tracing::warn!("audit_summary: line {i} parse failed: {e}"),
            }
        }
        Ok(s)
    }
}

fn outcome_key(o: &EgressOutcome) -> &'static str {
    match o {
        EgressOutcome::Ok => "ok",
        EgressOutcome::PolicyDenied => "policy_denied",
        EgressOutcome::SsrfBlocked => "ssrf_blocked",
        EgressOutcome::BodyOversized => "body_oversized",
        EgressOutcome::TlsError => "tls_error",
        EgressOutcome::Timeout => "timeout",
        EgressOutcome::UpstreamError => "upstream_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::baseline_egress_request;

    fn ev(host: &str, outcome: EgressOutcome) -> AuditEvent {
        let mut e = baseline_egress_request("native", "skill:test", "GET", host, "/x");
        e.outcome = outcome;
        e
    }

    #[test]
    fn empty_summary_is_zero() {
        let s = AuditSummary::from_events(&[]);
        assert_eq!(s.total_requests, 0);
        assert!(s.by_outcome.is_empty());
        assert!(s.by_host.is_empty());
    }

    #[test]
    fn ingest_single_ok_event_increments_correctly() {
        let mut s = AuditSummary::new();
        s.ingest(&ev("api.openai.com", EgressOutcome::Ok));
        assert_eq!(s.total_requests, 1);
        assert_eq!(s.by_outcome.get("ok"), Some(&1));
        assert_eq!(s.by_host.get("api.openai.com"), Some(&1));
    }

    #[test]
    fn mixed_outcomes_aggregate_separately() {
        let s = AuditSummary::from_events(&[
            ev("a.com", EgressOutcome::Ok),
            ev("a.com", EgressOutcome::Ok),
            ev("b.com", EgressOutcome::PolicyDenied),
            ev("10.0.0.1", EgressOutcome::SsrfBlocked),
        ]);
        assert_eq!(s.total_requests, 4);
        assert_eq!(s.by_outcome.get("ok"), Some(&2));
        assert_eq!(s.by_outcome.get("policy_denied"), Some(&1));
        assert_eq!(s.by_outcome.get("ssrf_blocked"), Some(&1));
        assert_eq!(s.by_host.get("a.com"), Some(&2));
    }

    #[test]
    fn inject_none_does_not_appear_in_by_inject() {
        let mut s = AuditSummary::new();
        s.ingest(&ev("a.com", EgressOutcome::Ok));
        assert!(s.by_inject.is_empty(),
            "default inject 'None' should not be counted");
    }

    #[test]
    fn inject_types_aggregate() {
        let mut e1 = ev("api.openai.com", EgressOutcome::Ok);
        e1.inject = "Bearer".into();
        let mut e2 = ev("api.openai.com", EgressOutcome::Ok);
        e2.inject = "Bearer".into();
        let mut e3 = ev("s3.amazonaws.com", EgressOutcome::Ok);
        e3.inject = "AwsSigV4".into();
        let s = AuditSummary::from_events(&[e1, e2, e3]);
        assert_eq!(s.by_inject.get("Bearer"), Some(&2));
        assert_eq!(s.by_inject.get("AwsSigV4"), Some(&1));
    }

    #[test]
    fn bytes_accumulate_across_events() {
        let mut a = ev("a.com", EgressOutcome::Ok);
        a.bytes_request = 100;
        a.bytes_response = 200;
        let mut b = ev("a.com", EgressOutcome::Ok);
        b.bytes_request = 50;
        b.bytes_response = 150;
        let s = AuditSummary::from_events(&[a, b]);
        assert_eq!(s.bytes_request_total, 150);
        assert_eq!(s.bytes_response_total, 350);
    }

    #[test]
    fn from_jsonl_file_round_trips() {
        use crate::audit::AuditSink;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let sink = crate::audit::JsonlFileAuditSink::open(&path).unwrap();
        sink.record(ev("a.com", EgressOutcome::Ok));
        sink.record(ev("b.com", EgressOutcome::PolicyDenied));
        drop(sink);
        let s = AuditSummary::from_jsonl_file(&path).unwrap();
        assert_eq!(s.total_requests, 2);
        assert_eq!(s.by_outcome.get("ok"), Some(&1));
        assert_eq!(s.by_outcome.get("policy_denied"), Some(&1));
    }

    #[test]
    fn from_jsonl_file_tolerates_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        // Mix one valid event and one garbage line.
        let valid = serde_json::to_string(&ev("a.com", EgressOutcome::Ok)).unwrap();
        std::fs::write(&path, format!("{valid}\nNOT_JSON\n")).unwrap();
        let s = AuditSummary::from_jsonl_file(&path).unwrap();
        assert_eq!(s.total_requests, 1);
    }
}
