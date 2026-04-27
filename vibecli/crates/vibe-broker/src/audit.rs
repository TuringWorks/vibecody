//! Audit-event types — emitted per egress request, consumed by the recap
//! subsystem (see `docs/design/recap-resume/02-job.md`).

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
}
