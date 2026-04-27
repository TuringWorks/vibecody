//! Audit-event types + sink trait — emitted per egress request, consumed
//! by the recap subsystem (see `docs/design/recap-resume/02-job.md`).
//!
//! The sink abstraction lets us route events to memory (tests),
//! JSONL files (production), or a tracing-subscriber JSON layer
//! (when the daemon is already configured for it). Broker hot paths
//! call `sink.record(...)` synchronously after each decision.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EgressOutcome {
    Ok,
    PolicyDenied,
    SsrfBlocked,
    BodyOversized,
    TlsError,
    Timeout,
    UpstreamError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub event: String,
    pub ts: String,
    pub session_id: Option<String>,
    pub policy_id: String,
    pub tier: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub status: Option<u16>,
    pub bytes_request: u64,
    pub bytes_response: u64,
    pub duration_ms: u64,
    pub inject: String,
    pub matched_rule_index: Option<usize>,
    pub user_consented: bool,
    pub outcome: EgressOutcome,
}

impl AuditEvent {
    pub fn egress_request() -> Self {
        AuditEvent {
            event: "egress.request".into(),
            ts: chrono_now_string(),
            session_id: None,
            policy_id: String::new(),
            tier: String::new(),
            host: String::new(),
            method: String::new(),
            path: String::new(),
            status: None,
            bytes_request: 0,
            bytes_response: 0,
            duration_ms: 0,
            inject: "None".into(),
            matched_rule_index: None,
            user_consented: false,
            outcome: EgressOutcome::Ok,
        }
    }
}

fn chrono_now_string() -> String {
    // Avoids the chrono dep in this crate; a coarse RFC-3339-ish string is
    // sufficient for v1 audit. Production callers can override `ts` before
    // emit.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

/// Where audit events go. Implementations are `Send + Sync` and must be
/// safe to call from any task without a separate runtime.
pub trait AuditSink: Send + Sync {
    fn record(&self, event: AuditEvent);
}

/// Default sink — drops every event. The broker holds one of these
/// unless the operator wires something real in.
#[derive(Debug, Default)]
pub struct NullAuditSink;

impl AuditSink for NullAuditSink {
    fn record(&self, _event: AuditEvent) {}
}

/// In-memory sink for tests. Holds every event for later assertion.
#[derive(Debug, Default)]
pub struct MemoryAuditSink {
    events: Mutex<Vec<AuditEvent>>,
}

impl MemoryAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the recorded events. Cheap clone of the inner Vec.
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Number of events recorded so far.
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// Drop everything. Useful between scenarios in BDD World structs.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl AuditSink for MemoryAuditSink {
    fn record(&self, event: AuditEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Helper for hot-path callers: build a baseline `egress.request` event,
/// pre-filled with what we already know at parse time. Callers mutate
/// `outcome`, `status`, byte counts before recording.
pub fn baseline_egress_request(
    tier: &str,
    policy_id: &str,
    method: &str,
    host: &str,
    path: &str,
) -> AuditEvent {
    AuditEvent {
        event: "egress.request".into(),
        ts: rfc3339_now(),
        session_id: None,
        policy_id: policy_id.into(),
        tier: tier.into(),
        host: host.into(),
        method: method.into(),
        path: path.into(),
        status: None,
        bytes_request: 0,
        bytes_response: 0,
        duration_ms: 0,
        inject: "None".into(),
        matched_rule_index: None,
        user_consented: false,
        outcome: EgressOutcome::Ok,
    }
}

fn rfc3339_now() -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let secs = d.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400) as u32;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day / 60) % 60;
    let ss = secs_of_day % 60;
    let (y, mo, day) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egress_request_has_event_name() {
        let e = AuditEvent::egress_request();
        assert_eq!(e.event, "egress.request");
        assert_eq!(e.outcome, EgressOutcome::Ok);
    }

    #[test]
    fn outcome_serializes_snake_case() {
        let s = serde_json::to_string(&EgressOutcome::PolicyDenied).unwrap();
        assert_eq!(s, "\"policy_denied\"");
    }

    #[test]
    fn memory_sink_records_in_order() {
        let sink = MemoryAuditSink::new();
        let mut a = baseline_egress_request("native", "skill:foo", "GET", "a.com", "/x");
        a.outcome = EgressOutcome::Ok;
        sink.record(a);
        let mut b = baseline_egress_request("native", "skill:bar", "POST", "b.com", "/y");
        b.outcome = EgressOutcome::PolicyDenied;
        sink.record(b);
        assert_eq!(sink.len(), 2);
        let events = sink.events();
        assert_eq!(events[0].outcome, EgressOutcome::Ok);
        assert_eq!(events[1].outcome, EgressOutcome::PolicyDenied);
    }

    #[test]
    fn null_sink_drops() {
        let sink = NullAuditSink;
        sink.record(AuditEvent::egress_request());
        // No assertion needed; the sink discards.
    }

    #[test]
    fn baseline_carries_method_host_path() {
        let e = baseline_egress_request("native", "skill:foo", "GET", "api.openai.com", "/v1");
        assert_eq!(e.method, "GET");
        assert_eq!(e.host, "api.openai.com");
        assert_eq!(e.path, "/v1");
        assert_eq!(e.policy_id, "skill:foo");
        assert_eq!(e.tier, "native");
        assert!(e.ts.starts_with('2'));
    }
}
