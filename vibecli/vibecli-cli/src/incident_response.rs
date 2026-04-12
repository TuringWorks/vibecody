//! AI-driven incident response automation.
//!
//! GAP-v9-018: rivals PagerDuty AIOps, Opsgenie AI, Amazon Q DevOps.
//! - Incident lifecycle: detection → triage → escalation → resolution → post-mortem
//! - Severity classification (P0–P4) with SLO breach prediction
//! - On-call routing with rotation schedule and escalation chains
//! - Runbook matching: keyword-based lookup of relevant remediation steps
//! - Root-cause hypothesis ranking (frequency × recency weighted)
//! - Automated timeline and post-mortem draft generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Severity ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Complete outage, revenue impact.
    P0,
    /// Major degradation, SLA at risk.
    P1,
    /// Partial degradation.
    P2,
    /// Minor issue, workaround available.
    P3,
    /// Cosmetic / future prevention.
    P4,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::P0 => "P0", Self::P1 => "P1", Self::P2 => "P2",
            Self::P3 => "P3", Self::P4 => "P4",
        })
    }
}

impl Severity {
    /// Maximum time to first response in minutes.
    pub fn response_slo_minutes(&self) -> u32 {
        match self { Self::P0 => 5, Self::P1 => 15, Self::P2 => 60, Self::P3 => 240, Self::P4 => 1440 }
    }

    /// Classify based on error rate and latency impact.
    pub fn classify(error_rate_pct: f64, latency_p99_ms: u64, affected_users_pct: f64) -> Self {
        if error_rate_pct > 50.0 || affected_users_pct > 75.0 { return Self::P0; }
        if error_rate_pct > 20.0 || latency_p99_ms > 5000 || affected_users_pct > 25.0 { return Self::P1; }
        if error_rate_pct > 5.0  || latency_p99_ms > 2000 { return Self::P2; }
        if error_rate_pct > 1.0  || latency_p99_ms > 1000 { return Self::P3; }
        Self::P4
    }
}

// ─── Incident ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentStatus {
    Detected, Acknowledged, Investigating, Mitigated, Resolved,
}

impl std::fmt::Display for IncidentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Detected      => "Detected",
            Self::Acknowledged  => "Acknowledged",
            Self::Investigating => "Investigating",
            Self::Mitigated     => "Mitigated",
            Self::Resolved      => "Resolved",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub ts_ms: u64,
    pub actor: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub status: IncidentStatus,
    pub detected_at_ms: u64,
    pub acknowledged_at_ms: Option<u64>,
    pub resolved_at_ms: Option<u64>,
    pub affected_services: Vec<String>,
    pub timeline: Vec<TimelineEvent>,
    pub labels: HashMap<String, String>,
}

impl Incident {
    pub fn new(id: impl Into<String>, title: impl Into<String>, severity: Severity, detected_at_ms: u64) -> Self {
        Self {
            id: id.into(), title: title.into(), severity, status: IncidentStatus::Detected,
            detected_at_ms, acknowledged_at_ms: None, resolved_at_ms: None,
            affected_services: Vec::new(), timeline: Vec::new(), labels: HashMap::new(),
        }
    }

    pub fn acknowledge(&mut self, ts_ms: u64, actor: impl Into<String>) {
        self.acknowledged_at_ms = Some(ts_ms);
        self.status = IncidentStatus::Acknowledged;
        self.add_event(ts_ms, actor, "Incident acknowledged");
    }

    pub fn resolve(&mut self, ts_ms: u64, actor: impl Into<String>, resolution: impl Into<String>) {
        self.resolved_at_ms = Some(ts_ms);
        self.status = IncidentStatus::Resolved;
        self.add_event(ts_ms, actor, resolution);
    }

    pub fn add_event(&mut self, ts_ms: u64, actor: impl Into<String>, description: impl Into<String>) {
        self.timeline.push(TimelineEvent { ts_ms, actor: actor.into(), description: description.into() });
    }

    /// Time to first acknowledgement in minutes.
    pub fn tta_minutes(&self) -> Option<u64> {
        self.acknowledged_at_ms.map(|a| (a - self.detected_at_ms) / 60_000)
    }

    /// Total duration from detection to resolution in minutes.
    pub fn ttr_minutes(&self) -> Option<u64> {
        self.resolved_at_ms.map(|r| (r - self.detected_at_ms) / 60_000)
    }

    /// Whether SLO for first response has been breached.
    pub fn slo_breached(&self, current_ts_ms: u64) -> bool {
        let elapsed_minutes = (current_ts_ms - self.detected_at_ms) / 60_000;
        match &self.acknowledged_at_ms {
            Some(ack) => (ack - self.detected_at_ms) / 60_000 > self.severity.response_slo_minutes() as u64,
            None => elapsed_minutes > self.severity.response_slo_minutes() as u64,
        }
    }
}

// ─── On-Call Rotation ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallSlot {
    pub engineer: String,
    /// Slot start offset in hours from rotation epoch.
    pub start_hour: u64,
    pub duration_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallRotation {
    pub service: String,
    pub slots: Vec<OnCallSlot>,
    /// Escalation chain: first element = primary on-call, subsequent = escalation levels.
    pub escalation_chain: Vec<String>,
}

impl OnCallRotation {
    /// Who is on-call at `hour_offset` hours from epoch?
    pub fn on_call_at(&self, hour_offset: u64) -> Option<&str> {
        for slot in &self.slots {
            if hour_offset >= slot.start_hour && hour_offset < slot.start_hour + slot.duration_hours {
                return Some(&slot.engineer);
            }
        }
        None
    }

    /// Escalation chain starting with the current on-call engineer.
    pub fn escalate(&self, hour_offset: u64) -> Vec<&str> {
        let primary = self.on_call_at(hour_offset);
        let mut chain: Vec<&str> = primary.into_iter().collect();
        for e in &self.escalation_chain {
            if primary != Some(e.as_str()) { chain.push(e.as_str()); }
        }
        chain
    }
}

// ─── Runbook ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runbook {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
    pub steps: Vec<String>,
    pub services: Vec<String>,
}

impl Runbook {
    pub fn matches(&self, incident: &Incident) -> f64 {
        let mut score = 0.0_f64;
        let title_lower = incident.title.to_lowercase();
        for kw in &self.keywords {
            if title_lower.contains(kw.to_lowercase().as_str()) { score += 1.0; }
        }
        for svc in &incident.affected_services {
            if self.services.contains(svc) { score += 2.0; }
        }
        score
    }
}

// ─── Root-Cause Hypothesis ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub description: String,
    /// Number of times this cause appeared in past incidents.
    pub frequency: u32,
    /// Hours since last occurrence (lower = more recent).
    pub hours_since_last: u32,
    pub confidence: f64,
}

impl Hypothesis {
    /// Rank score: frequency / log2(hours_since_last + 2) — higher is more likely.
    pub fn rank_score(&self) -> f64 {
        let recency = (self.hours_since_last as f64 + 2.0).log2();
        self.frequency as f64 / recency
    }
}

/// Sort hypotheses by rank score descending.
pub fn rank_hypotheses(mut hypotheses: Vec<Hypothesis>) -> Vec<Hypothesis> {
    hypotheses.sort_by(|a, b| b.rank_score().partial_cmp(&a.rank_score()).unwrap_or(std::cmp::Ordering::Equal));
    hypotheses
}

// ─── Incident Manager ────────────────────────────────────────────────────────

pub struct IncidentManager {
    pub incidents: HashMap<String, Incident>,
    pub runbooks: Vec<Runbook>,
    pub rotations: HashMap<String, OnCallRotation>,
}

impl IncidentManager {
    pub fn new() -> Self {
        Self { incidents: HashMap::new(), runbooks: Vec::new(), rotations: HashMap::new() }
    }

    pub fn open_incident(&mut self, incident: Incident) {
        self.incidents.insert(incident.id.clone(), incident);
    }

    pub fn add_runbook(&mut self, rb: Runbook) { self.runbooks.push(rb); }
    pub fn add_rotation(&mut self, rot: OnCallRotation) { self.rotations.insert(rot.service.clone(), rot); }

    /// Find runbooks relevant to this incident, sorted by match score.
    pub fn matching_runbooks(&self, incident: &Incident) -> Vec<(&Runbook, f64)> {
        let mut scored: Vec<_> = self.runbooks.iter()
            .map(|rb| (rb, rb.matches(incident)))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Generate a post-mortem draft for a resolved incident.
    pub fn post_mortem(&self, incident_id: &str) -> Option<String> {
        let inc = self.incidents.get(incident_id)?;
        if inc.status != IncidentStatus::Resolved { return None; }

        let ttd = "at detection";
        let tta = inc.tta_minutes().map(|m| format!("{m} min")).unwrap_or_else(|| "unknown".into());
        let ttr = inc.ttr_minutes().map(|m| format!("{m} min")).unwrap_or_else(|| "unknown".into());

        let events: Vec<String> = inc.timeline.iter().map(|e| {
            format!("  - [{}ms] {}: {}", e.ts_ms, e.actor, e.description)
        }).collect();

        let runbooks: Vec<String> = self.matching_runbooks(inc)
            .into_iter().take(3).map(|(rb, _)| format!("  - {} ({})", rb.title, rb.id)).collect();

        Some(format!(
            "# Post-Mortem: {} ({})\n\n\
             **Severity**: {}\n\
             **Detected**: {}\n\
             **Time to Acknowledge**: {}\n\
             **Time to Resolve**: {}\n\n\
             ## Timeline\n{}\n\n\
             ## Relevant Runbooks\n{}\n\n\
             ## Action Items\n- [ ] Root cause confirmed\n- [ ] Fix deployed\n- [ ] Monitoring improved\n",
            inc.title, inc.id, inc.severity, ttd, tta, ttr,
            events.join("\n"),
            if runbooks.is_empty() { "  - None matched".into() } else { runbooks.join("\n") }
        ))
    }

    /// List incidents that breached their SLO at the given timestamp.
    pub fn slo_breaches(&self, current_ts_ms: u64) -> Vec<&Incident> {
        self.incidents.values().filter(|i| i.slo_breached(current_ts_ms)).collect()
    }
}

impl Default for IncidentManager {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn base_incident() -> Incident {
        let mut i = Incident::new("INC-001", "Database connection timeout", Severity::P1, 0);
        i.affected_services = vec!["db".into(), "api".into()];
        i
    }

    #[test]
    fn test_severity_classify_p0() {
        assert_eq!(Severity::classify(60.0, 100, 10.0), Severity::P0);
        assert_eq!(Severity::classify(10.0, 100, 80.0), Severity::P0);
    }

    #[test]
    fn test_severity_classify_p1() {
        assert_eq!(Severity::classify(25.0, 100, 10.0), Severity::P1);
        assert_eq!(Severity::classify(5.0, 6000, 10.0), Severity::P1);
    }

    #[test]
    fn test_severity_classify_p2() {
        assert_eq!(Severity::classify(8.0, 500, 5.0), Severity::P2);
    }

    #[test]
    fn test_severity_classify_p3() {
        assert_eq!(Severity::classify(1.5, 1100, 0.5), Severity::P3);
    }

    #[test]
    fn test_severity_classify_p4() {
        assert_eq!(Severity::classify(0.1, 100, 0.1), Severity::P4);
    }

    #[test]
    fn test_severity_response_slo() {
        assert_eq!(Severity::P0.response_slo_minutes(), 5);
        assert_eq!(Severity::P1.response_slo_minutes(), 15);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::P0.to_string(), "P0");
        assert_eq!(Severity::P3.to_string(), "P3");
    }

    #[test]
    fn test_incident_acknowledge() {
        let mut i = base_incident();
        i.acknowledge(5 * 60_000, "alice");
        assert_eq!(i.status, IncidentStatus::Acknowledged);
        assert_eq!(i.tta_minutes(), Some(5));
    }

    #[test]
    fn test_incident_resolve() {
        let mut i = base_incident();
        i.acknowledge(5 * 60_000, "alice");
        i.resolve(30 * 60_000, "bob", "Restarted db connection pool");
        assert_eq!(i.status, IncidentStatus::Resolved);
        assert_eq!(i.ttr_minutes(), Some(30));
    }

    #[test]
    fn test_slo_not_breached_within_window() {
        let i = base_incident(); // P1, SLO=15 min
        let current = 10 * 60_000; // 10 min elapsed, not yet breached
        assert!(!i.slo_breached(current));
    }

    #[test]
    fn test_slo_breached_when_unacked() {
        let i = base_incident(); // P1, SLO=15 min
        let current = 20 * 60_000; // 20 min elapsed without ack
        assert!(i.slo_breached(current));
    }

    #[test]
    fn test_slo_breached_slow_ack() {
        let mut i = base_incident(); // P1, SLO=15 min
        i.acknowledge(20 * 60_000, "alice"); // ack at 20 min > 15 min SLO
        assert!(i.slo_breached(0));
    }

    #[test]
    fn test_oncall_slot_lookup() {
        let rot = OnCallRotation {
            service: "api".into(),
            slots: vec![
                OnCallSlot { engineer: "alice".into(), start_hour: 0, duration_hours: 12 },
                OnCallSlot { engineer: "bob".into(),   start_hour: 12, duration_hours: 12 },
            ],
            escalation_chain: vec!["manager".into()],
        };
        assert_eq!(rot.on_call_at(5), Some("alice"));
        assert_eq!(rot.on_call_at(14), Some("bob"));
        assert_eq!(rot.on_call_at(25), None);
    }

    #[test]
    fn test_oncall_escalation_includes_manager() {
        let rot = OnCallRotation {
            service: "api".into(),
            slots: vec![OnCallSlot { engineer: "alice".into(), start_hour: 0, duration_hours: 24 }],
            escalation_chain: vec!["manager".into()],
        };
        let chain = rot.escalate(0);
        assert_eq!(chain[0], "alice");
        assert!(chain.contains(&"manager"));
    }

    #[test]
    fn test_runbook_matches_by_keyword() {
        let rb = Runbook {
            id: "rb-001".into(), title: "DB Connection".into(),
            keywords: vec!["timeout".into(), "database".into()],
            steps: vec!["Check pool".into()], services: vec![],
        };
        let i = base_incident(); // title: "Database connection timeout"
        assert!(rb.matches(&i) > 0.0);
    }

    #[test]
    fn test_runbook_matches_by_service() {
        let rb = Runbook {
            id: "rb-002".into(), title: "API Health".into(),
            keywords: vec![], steps: vec![],
            services: vec!["api".into()],
        };
        let i = base_incident();
        assert!(rb.matches(&i) > 0.0);
    }

    #[test]
    fn test_runbook_no_match() {
        let rb = Runbook {
            id: "rb-003".into(), title: "Deploy".into(),
            keywords: vec!["deployment".to_string()], steps: vec![], services: vec!["deploy-service".into()],
        };
        let i = base_incident();
        assert_eq!(rb.matches(&i), 0.0);
    }

    #[test]
    fn test_hypothesis_rank_recent_wins() {
        let h1 = Hypothesis { description: "Cache miss".into(), frequency: 10, hours_since_last: 1, confidence: 0.8 };
        let h2 = Hypothesis { description: "OOM".into(), frequency: 10, hours_since_last: 100, confidence: 0.5 };
        assert!(h1.rank_score() > h2.rank_score());
    }

    #[test]
    fn test_hypothesis_rank_frequent_wins_same_recency() {
        let h1 = Hypothesis { description: "A".into(), frequency: 20, hours_since_last: 5, confidence: 0.9 };
        let h2 = Hypothesis { description: "B".into(), frequency: 5, hours_since_last: 5, confidence: 0.5 };
        assert!(h1.rank_score() > h2.rank_score());
    }

    #[test]
    fn test_rank_hypotheses_sorted() {
        let hyps = vec![
            Hypothesis { description: "B".into(), frequency: 2, hours_since_last: 5, confidence: 0.3 },
            Hypothesis { description: "A".into(), frequency: 20, hours_since_last: 1, confidence: 0.9 },
        ];
        let ranked = rank_hypotheses(hyps);
        assert_eq!(ranked[0].description, "A");
    }

    #[test]
    fn test_post_mortem_draft_generated() {
        let mut mgr = IncidentManager::new();
        let mut i = base_incident();
        i.acknowledge(5 * 60_000, "alice");
        i.resolve(30 * 60_000, "bob", "Fixed");
        mgr.open_incident(i);
        let pm = mgr.post_mortem("INC-001");
        assert!(pm.is_some());
        let text = pm.unwrap();
        assert!(text.contains("Post-Mortem"));
        assert!(text.contains("P1"));
        assert!(text.contains("30 min"));
    }

    #[test]
    fn test_post_mortem_none_for_open_incident() {
        let mut mgr = IncidentManager::new();
        mgr.open_incident(base_incident());
        assert!(mgr.post_mortem("INC-001").is_none());
    }

    #[test]
    fn test_slo_breach_detection() {
        let mut mgr = IncidentManager::new();
        mgr.open_incident(base_incident()); // P1, 15 min SLO
        let breached = mgr.slo_breaches(20 * 60_000);
        assert_eq!(breached.len(), 1);
    }

    #[test]
    fn test_no_slo_breach_early() {
        let mut mgr = IncidentManager::new();
        mgr.open_incident(base_incident());
        let breached = mgr.slo_breaches(5 * 60_000);
        assert!(breached.is_empty());
    }

    #[test]
    fn test_matching_runbooks_sorted() {
        let mut mgr = IncidentManager::new();
        mgr.add_runbook(Runbook {
            id: "rb-a".into(), title: "Weak".into(),
            keywords: vec!["timeout".into()], steps: vec![], services: vec![],
        });
        mgr.add_runbook(Runbook {
            id: "rb-b".into(), title: "Strong".into(),
            keywords: vec!["timeout".into(), "database".into()],
            steps: vec![], services: vec!["db".into()],
        });
        let i = base_incident();
        let matches = mgr.matching_runbooks(&i);
        assert_eq!(matches[0].0.id, "rb-b");
    }

    #[test]
    fn test_incident_timeline_recorded() {
        let mut i = base_incident();
        i.add_event(1000, "monitor", "Alert fired");
        i.add_event(2000, "alice", "Looking into it");
        assert_eq!(i.timeline.len(), 2);
        assert_eq!(i.timeline[0].actor, "monitor");
    }

    #[test]
    fn test_incident_status_display() {
        assert_eq!(IncidentStatus::Acknowledged.to_string(), "Acknowledged");
        assert_eq!(IncidentStatus::Resolved.to_string(), "Resolved");
    }
}
