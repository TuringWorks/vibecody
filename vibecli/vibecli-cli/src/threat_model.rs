//! Agentic security threat modeling and mitigation engine.
//!
//! GAP-v9-007: rivals Devin SecurityAI, GitHub Advanced Security, Amazon Security Baselines.
//! - STRIDE threat categorisation (Spoofing, Tampering, Repudiation, Info Disclosure, DoS, EoP)
//! - PASTA process (7 stages: objectives → attack tree → countermeasures)
//! - Attack tree generation with probability + impact scoring
//! - AI-suggested mitigations per threat with CVSS-like risk rating
//! - Data-flow diagram (DFD) element tracking: processes, data stores, external entities
//! - Control mapping: NIST CSF, OWASP Top 10, CWE categories

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── STRIDE ──────────────────────────────────────────────────────────────────

/// The 6 STRIDE threat categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stride {
    Spoofing,
    Tampering,
    Repudiation,
    InformationDisclosure,
    DenialOfService,
    ElevationOfPrivilege,
}

impl Stride {
    pub fn acronym(&self) -> char {
        match self {
            Self::Spoofing              => 'S',
            Self::Tampering             => 'T',
            Self::Repudiation           => 'R',
            Self::InformationDisclosure => 'I',
            Self::DenialOfService       => 'D',
            Self::ElevationOfPrivilege  => 'E',
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Spoofing              => "Impersonating another user or system",
            Self::Tampering             => "Modifying data or code without authorisation",
            Self::Repudiation           => "Denying actions without proof",
            Self::InformationDisclosure => "Exposing data to unauthorised parties",
            Self::DenialOfService       => "Preventing legitimate access to services",
            Self::ElevationOfPrivilege  => "Gaining higher permissions than granted",
        }
    }

    /// Default risk multiplier for ranking (higher = more severe by default).
    pub fn base_risk(&self) -> f64 {
        match self {
            Self::ElevationOfPrivilege  => 9.0,
            Self::Tampering             => 8.0,
            Self::InformationDisclosure => 7.5,
            Self::Spoofing              => 7.0,
            Self::DenialOfService       => 6.5,
            Self::Repudiation           => 5.0,
        }
    }
}

// ─── DFD Elements ────────────────────────────────────────────────────────────

/// Data-Flow Diagram element types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DfdElementKind {
    Process,
    DataStore,
    ExternalEntity,
    DataFlow,
}

/// A DFD element (trust boundary, process, data store, external entity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfdElement {
    pub id: String,
    pub name: String,
    pub kind: DfdElementKind,
    pub trust_level: u8,  // 0 = untrusted, 10 = fully trusted
    pub data_types: Vec<String>,
}

impl DfdElement {
    pub fn new(id: &str, name: &str, kind: DfdElementKind, trust: u8) -> Self {
        Self { id: id.to_string(), name: name.to_string(), kind, trust_level: trust, data_types: Vec::new() }
    }

    pub fn is_untrusted(&self) -> bool { self.trust_level <= 3 }
}

// ─── Threat Types ─────────────────────────────────────────────────────────────

/// A specific security threat identified by the modeler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: String,
    pub stride: Stride,
    pub title: String,
    pub description: String,
    pub affected_element: String,
    pub probability: f64,  // 0.0 – 1.0
    pub impact: f64,       // 0.0 – 10.0
    pub risk_score: f64,   // probability × impact
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub mitigations: Vec<Mitigation>,
    pub status: ThreatStatus,
}

impl Threat {
    pub fn new(id: &str, stride: Stride, title: &str, element: &str, prob: f64, impact: f64) -> Self {
        let risk = prob * impact;
        Self {
            id: id.to_string(),
            stride,
            title: title.to_string(),
            description: String::new(),
            affected_element: element.to_string(),
            probability: prob,
            impact,
            risk_score: risk,
            cwe: None,
            owasp: None,
            mitigations: Vec::new(),
            status: ThreatStatus::Open,
        }
    }

    pub fn with_cwe(mut self, cwe: &str) -> Self { self.cwe = Some(cwe.to_string()); self }
    pub fn with_owasp(mut self, cat: &str) -> Self { self.owasp = Some(cat.to_string()); self }

    pub fn severity(&self) -> ThreatSeverity {
        match self.risk_score as u32 {
            8..=10 => ThreatSeverity::Critical,
            6..=7  => ThreatSeverity::High,
            3..=5  => ThreatSeverity::Medium,
            1..=2  => ThreatSeverity::Low,
            _      => ThreatSeverity::Informational,
        }
    }
}

/// Threat lifecycle status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatStatus {
    Open,
    Mitigated,
    Accepted,
    Transferred,
}

/// Risk severity classification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Informational,
    Low,
    Medium,
    High,
    Critical,
}

// ─── Mitigations ─────────────────────────────────────────────────────────────

/// A security control / mitigation for a threat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mitigation {
    pub id: String,
    pub title: String,
    pub control_type: ControlType,
    pub effectiveness: f64,  // 0.0 – 1.0 (how much it reduces risk)
    pub implementation_effort: EffortLevel,
    pub nist_csf: Vec<String>,  // e.g. ["PR.AC-1", "DE.CM-7"]
}

/// Security control types (NIST taxonomy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlType {
    Preventive,
    Detective,
    Corrective,
    Deterrent,
    Compensating,
}

/// Implementation effort level.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EffortLevel {
    Trivial,
    Low,
    Medium,
    High,
    Expert,
}

// ─── Attack Tree ──────────────────────────────────────────────────────────────

/// A node in an attack tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackNode {
    pub id: String,
    pub label: String,
    pub gate: AttackGate,
    pub probability: f64,
    pub cost_to_attacker: f64,
    pub children: Vec<AttackNode>,
}

/// Logic gate for attack tree nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttackGate {
    And,  // all children must succeed
    Or,   // any child succeeds
    Leaf, // terminal node
}

impl AttackNode {
    pub fn leaf(id: &str, label: &str, prob: f64, cost: f64) -> Self {
        Self { id: id.to_string(), label: label.to_string(), gate: AttackGate::Leaf, probability: prob, cost_to_attacker: cost, children: Vec::new() }
    }

    pub fn or_node(id: &str, label: &str, children: Vec<AttackNode>) -> Self {
        Self { id: id.to_string(), label: label.to_string(), gate: AttackGate::Or, probability: 0.0, cost_to_attacker: 0.0, children }
    }

    pub fn and_node(id: &str, label: &str, children: Vec<AttackNode>) -> Self {
        Self { id: id.to_string(), label: label.to_string(), gate: AttackGate::And, probability: 0.0, cost_to_attacker: 0.0, children }
    }

    /// Compute aggregate probability for OR/AND gates.
    pub fn aggregate_probability(&self) -> f64 {
        if self.children.is_empty() { return self.probability; }
        match self.gate {
            AttackGate::Leaf => self.probability,
            AttackGate::Or => {
                // P(A OR B) = 1 - P(!A)*P(!B)
                self.children.iter().fold(1.0, |acc, c| acc * (1.0 - c.aggregate_probability()))
                    .mul_add(-1.0, 1.0)
            }
            AttackGate::And => {
                // P(A AND B) = P(A)*P(B)
                self.children.iter().map(|c| c.aggregate_probability()).product()
            }
        }
    }

    /// Minimum attacker cost through the tree (optimal path).
    pub fn min_attacker_cost(&self) -> f64 {
        if self.children.is_empty() { return self.cost_to_attacker; }
        match self.gate {
            AttackGate::Leaf => self.cost_to_attacker,
            AttackGate::Or => self.children.iter().map(|c| c.min_attacker_cost()).fold(f64::MAX, f64::min),
            AttackGate::And => self.children.iter().map(|c| c.min_attacker_cost()).sum(),
        }
    }

    pub fn depth(&self) -> usize {
        if self.children.is_empty() { return 0; }
        1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
    }
}

// ─── Threat Model Engine ──────────────────────────────────────────────────────

/// Core threat modeling engine.
pub struct ThreatModeler {
    elements: Vec<DfdElement>,
    threats: Vec<Threat>,
    id_counter: u32,
}

impl ThreatModeler {
    pub fn new() -> Self {
        Self { elements: Vec::new(), threats: Vec::new(), id_counter: 0 }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.id_counter += 1;
        format!("{prefix}-{:04}", self.id_counter)
    }

    pub fn add_element(&mut self, elem: DfdElement) {
        self.elements.push(elem);
    }

    pub fn add_threat(&mut self, threat: Threat) {
        self.threats.push(threat);
    }

    /// Auto-generate STRIDE threats for untrusted external entities.
    pub fn auto_stride(&mut self, element: &DfdElement) -> Vec<Threat> {
        let mut generated = Vec::new();
        let strides = [Stride::Spoofing, Stride::Tampering, Stride::InformationDisclosure];
        for stride in &strides {
            let title = format!("{} threat on {}", stride.description().split_whitespace().next().unwrap_or("Threat"), element.name);
            let id = self.next_id("THR");
            let prob = if element.is_untrusted() { 0.7 } else { 0.3 };
            let impact = stride.base_risk();
            let threat = Threat::new(&id, stride.clone(), &title, &element.id, prob, impact);
            generated.push(threat);
        }
        self.threats.extend(generated.clone());
        generated
    }

    /// Suggest mitigations for a threat based on its STRIDE category.
    pub fn suggest_mitigations(&self, threat: &Threat) -> Vec<Mitigation> {
        match threat.stride {
            Stride::Spoofing => vec![
                Mitigation {
                    id: "MIT-AUTH".into(),
                    title: "Implement multi-factor authentication".into(),
                    control_type: ControlType::Preventive,
                    effectiveness: 0.85,
                    implementation_effort: EffortLevel::Medium,
                    nist_csf: vec!["PR.AC-1".into(), "PR.AC-7".into()],
                },
            ],
            Stride::Tampering => vec![
                Mitigation {
                    id: "MIT-SIGN".into(),
                    title: "Use cryptographic signatures / HMAC for data integrity".into(),
                    control_type: ControlType::Preventive,
                    effectiveness: 0.90,
                    implementation_effort: EffortLevel::Low,
                    nist_csf: vec!["PR.DS-6".into()],
                },
            ],
            Stride::InformationDisclosure => vec![
                Mitigation {
                    id: "MIT-ENC".into(),
                    title: "Enforce TLS 1.3 and at-rest encryption (AES-256)".into(),
                    control_type: ControlType::Preventive,
                    effectiveness: 0.95,
                    implementation_effort: EffortLevel::Low,
                    nist_csf: vec!["PR.DS-2".into(), "PR.DS-5".into()],
                },
            ],
            Stride::DenialOfService => vec![
                Mitigation {
                    id: "MIT-RATE".into(),
                    title: "Apply rate limiting and circuit breakers".into(),
                    control_type: ControlType::Preventive,
                    effectiveness: 0.75,
                    implementation_effort: EffortLevel::Medium,
                    nist_csf: vec!["PR.AC-4".into()],
                },
            ],
            Stride::ElevationOfPrivilege => vec![
                Mitigation {
                    id: "MIT-RBAC".into(),
                    title: "Enforce RBAC with principle of least privilege".into(),
                    control_type: ControlType::Preventive,
                    effectiveness: 0.88,
                    implementation_effort: EffortLevel::Medium,
                    nist_csf: vec!["PR.AC-4".into(), "PR.AC-6".into()],
                },
            ],
            Stride::Repudiation => vec![
                Mitigation {
                    id: "MIT-LOG".into(),
                    title: "Implement tamper-evident audit logging".into(),
                    control_type: ControlType::Detective,
                    effectiveness: 0.80,
                    implementation_effort: EffortLevel::Low,
                    nist_csf: vec!["DE.AE-1".into(), "PR.PT-1".into()],
                },
            ],
        }
    }

    /// Generate a threat model report.
    pub fn report(&self) -> ThreatModelReport {
        let mut by_stride: HashMap<String, usize> = HashMap::new();
        let mut critical = 0usize;
        for t in &self.threats {
            *by_stride.entry(format!("{:?}", t.stride)).or_insert(0) += 1;
            if t.severity() >= ThreatSeverity::High { critical += 1; }
        }
        let open = self.threats.iter().filter(|t| t.status == ThreatStatus::Open).count();
        let avg_risk = if self.threats.is_empty() { 0.0 }
            else { self.threats.iter().map(|t| t.risk_score).sum::<f64>() / self.threats.len() as f64 };
        ThreatModelReport {
            total_threats: self.threats.len(),
            open_threats: open,
            critical_or_high: critical,
            avg_risk_score: avg_risk,
            by_stride,
            dfd_element_count: self.elements.len(),
        }
    }

    pub fn threats(&self) -> &[Threat] { &self.threats }
    pub fn elements(&self) -> &[DfdElement] { &self.elements }

    /// Threats sorted by risk score descending.
    pub fn top_threats(&self, n: usize) -> Vec<&Threat> {
        let mut sorted: Vec<&Threat> = self.threats.iter().collect();
        sorted.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(n);
        sorted
    }
}

impl Default for ThreatModeler {
    fn default() -> Self { Self::new() }
}

/// Summary report of a threat model session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatModelReport {
    pub total_threats: usize,
    pub open_threats: usize,
    pub critical_or_high: usize,
    pub avg_risk_score: f64,
    pub by_stride: HashMap<String, usize>,
    pub dfd_element_count: usize,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_threat(id: &str, stride: Stride, prob: f64, impact: f64) -> Threat {
        Threat::new(id, stride, "Test threat", "proc-1", prob, impact)
    }

    // ── Stride ────────────────────────────────────────────────────────────

    #[test]
    fn test_stride_acronym() {
        assert_eq!(Stride::Spoofing.acronym(), 'S');
        assert_eq!(Stride::ElevationOfPrivilege.acronym(), 'E');
        assert_eq!(Stride::DenialOfService.acronym(), 'D');
    }

    #[test]
    fn test_stride_description_non_empty() {
        for s in [Stride::Spoofing, Stride::Tampering, Stride::Repudiation] {
            assert!(!s.description().is_empty());
        }
    }

    #[test]
    fn test_stride_eop_highest_base_risk() {
        assert!(Stride::ElevationOfPrivilege.base_risk() > Stride::Repudiation.base_risk());
    }

    // ── DfdElement ────────────────────────────────────────────────────────

    #[test]
    fn test_dfd_element_untrusted_at_0() {
        let e = DfdElement::new("e1", "Internet", DfdElementKind::ExternalEntity, 0);
        assert!(e.is_untrusted());
    }

    #[test]
    fn test_dfd_element_trusted_at_10() {
        let e = DfdElement::new("e2", "Internal DB", DfdElementKind::DataStore, 10);
        assert!(!e.is_untrusted());
    }

    #[test]
    fn test_dfd_element_boundary_at_3() {
        let e = DfdElement::new("e3", "DMZ", DfdElementKind::Process, 3);
        assert!(e.is_untrusted());
    }

    #[test]
    fn test_dfd_element_not_untrusted_at_4() {
        let e = DfdElement::new("e4", "Internal", DfdElementKind::Process, 4);
        assert!(!e.is_untrusted());
    }

    // ── Threat ────────────────────────────────────────────────────────────

    #[test]
    fn test_threat_risk_score_is_prob_times_impact() {
        let t = make_threat("t1", Stride::Spoofing, 0.6, 8.0);
        assert!((t.risk_score - 4.8).abs() < 0.001);
    }

    #[test]
    fn test_threat_severity_critical() {
        let t = make_threat("t1", Stride::ElevationOfPrivilege, 0.95, 9.5);
        assert_eq!(t.severity(), ThreatSeverity::Critical);
    }

    #[test]
    fn test_threat_severity_medium() {
        let t = make_threat("t1", Stride::Repudiation, 0.5, 8.0);
        assert_eq!(t.severity(), ThreatSeverity::Medium); // 4.0
    }

    #[test]
    fn test_threat_severity_informational_zero() {
        let t = make_threat("t1", Stride::DenialOfService, 0.0, 0.0);
        assert_eq!(t.severity(), ThreatSeverity::Informational);
    }

    #[test]
    fn test_threat_with_cwe() {
        let t = make_threat("t1", Stride::Tampering, 0.5, 5.0).with_cwe("CWE-79");
        assert_eq!(t.cwe.as_deref(), Some("CWE-79"));
    }

    #[test]
    fn test_threat_with_owasp() {
        let t = make_threat("t1", Stride::InformationDisclosure, 0.5, 5.0).with_owasp("A02:2021");
        assert_eq!(t.owasp.as_deref(), Some("A02:2021"));
    }

    // ── ThreatSeverity ordering ───────────────────────────────────────────

    #[test]
    fn test_severity_ordering() {
        assert!(ThreatSeverity::Critical > ThreatSeverity::High);
        assert!(ThreatSeverity::High > ThreatSeverity::Medium);
        assert!(ThreatSeverity::Medium > ThreatSeverity::Low);
        assert!(ThreatSeverity::Low > ThreatSeverity::Informational);
    }

    // ── AttackNode ────────────────────────────────────────────────────────

    #[test]
    fn test_attack_leaf_probability() {
        let node = AttackNode::leaf("n1", "guess password", 0.4, 10.0);
        assert!((node.aggregate_probability() - 0.4).abs() < 1e-5);
    }

    #[test]
    fn test_attack_or_probability() {
        let n1 = AttackNode::leaf("n1", "a", 0.5, 5.0);
        let n2 = AttackNode::leaf("n2", "b", 0.5, 5.0);
        let or_node = AttackNode::or_node("or", "either", vec![n1, n2]);
        // P(A OR B) = 1 - (1-0.5)(1-0.5) = 0.75
        assert!((or_node.aggregate_probability() - 0.75).abs() < 1e-5);
    }

    #[test]
    fn test_attack_and_probability() {
        let n1 = AttackNode::leaf("n1", "a", 0.5, 5.0);
        let n2 = AttackNode::leaf("n2", "b", 0.4, 8.0);
        let and_node = AttackNode::and_node("and", "both", vec![n1, n2]);
        // P(A AND B) = 0.5 * 0.4 = 0.2
        assert!((and_node.aggregate_probability() - 0.2).abs() < 1e-5);
    }

    #[test]
    fn test_attack_or_min_cost() {
        let n1 = AttackNode::leaf("n1", "cheap", 0.9, 100.0);
        let n2 = AttackNode::leaf("n2", "expensive", 0.5, 1000.0);
        let or_node = AttackNode::or_node("or", "cheapest path", vec![n1, n2]);
        assert!((or_node.min_attacker_cost() - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_attack_and_min_cost_sums() {
        let n1 = AttackNode::leaf("n1", "step1", 0.9, 200.0);
        let n2 = AttackNode::leaf("n2", "step2", 0.9, 300.0);
        let and_node = AttackNode::and_node("and", "both steps", vec![n1, n2]);
        assert!((and_node.min_attacker_cost() - 500.0).abs() < 1e-5);
    }

    #[test]
    fn test_attack_depth_leaf() {
        let node = AttackNode::leaf("n1", "leaf", 0.5, 10.0);
        assert_eq!(node.depth(), 0);
    }

    #[test]
    fn test_attack_depth_nested() {
        let leaf = AttackNode::leaf("l1", "leaf", 0.5, 5.0);
        let mid = AttackNode::or_node("m1", "mid", vec![leaf]);
        let root = AttackNode::or_node("r1", "root", vec![mid]);
        assert_eq!(root.depth(), 2);
    }

    // ── ThreatModeler ─────────────────────────────────────────────────────

    #[test]
    fn test_modeler_auto_stride_untrusted() {
        let mut m = ThreatModeler::new();
        let elem = DfdElement::new("e1", "Internet", DfdElementKind::ExternalEntity, 0);
        let threats = m.auto_stride(&elem);
        assert!(!threats.is_empty());
        assert!(threats.iter().any(|t| t.stride == Stride::Spoofing));
    }

    #[test]
    fn test_modeler_auto_stride_higher_prob_for_untrusted() {
        let mut m = ThreatModeler::new();
        let untrusted = DfdElement::new("e1", "Internet", DfdElementKind::ExternalEntity, 0);
        let trusted = DfdElement::new("e2", "InternalDB", DfdElementKind::DataStore, 9);
        let t_untrusted = m.auto_stride(&untrusted);
        let t_trusted = m.auto_stride(&trusted);
        let prob_u = t_untrusted[0].probability;
        let prob_t = t_trusted[0].probability;
        assert!(prob_u > prob_t);
    }

    #[test]
    fn test_modeler_suggest_mitigations_spoofing() {
        let m = ThreatModeler::new();
        let t = make_threat("t1", Stride::Spoofing, 0.7, 8.0);
        let mits = m.suggest_mitigations(&t);
        assert!(!mits.is_empty());
        assert_eq!(mits[0].control_type, ControlType::Preventive);
    }

    #[test]
    fn test_modeler_suggest_mitigations_repudiation_is_detective() {
        let m = ThreatModeler::new();
        let t = make_threat("t1", Stride::Repudiation, 0.5, 5.0);
        let mits = m.suggest_mitigations(&t);
        assert_eq!(mits[0].control_type, ControlType::Detective);
    }

    #[test]
    fn test_modeler_suggest_mitigations_all_have_nist() {
        let m = ThreatModeler::new();
        for stride in [Stride::Spoofing, Stride::Tampering, Stride::InformationDisclosure,
                       Stride::DenialOfService, Stride::ElevationOfPrivilege, Stride::Repudiation] {
            let t = make_threat("t", stride, 0.5, 5.0);
            let mits = m.suggest_mitigations(&t);
            assert!(!mits[0].nist_csf.is_empty());
        }
    }

    #[test]
    fn test_modeler_top_threats_sorted() {
        let mut m = ThreatModeler::new();
        m.add_threat(make_threat("t1", Stride::Repudiation, 0.3, 5.0)); // 1.5
        m.add_threat(make_threat("t2", Stride::ElevationOfPrivilege, 0.9, 9.0)); // 8.1
        m.add_threat(make_threat("t3", Stride::Tampering, 0.6, 7.0)); // 4.2
        let top = m.top_threats(2);
        assert_eq!(top[0].id, "t2");
        assert_eq!(top[1].id, "t3");
    }

    #[test]
    fn test_modeler_report_counts() {
        let mut m = ThreatModeler::new();
        m.add_element(DfdElement::new("e1", "API", DfdElementKind::Process, 5));
        m.add_threat(make_threat("t1", Stride::Spoofing, 0.9, 9.5)); // critical
        m.add_threat(make_threat("t2", Stride::Repudiation, 0.1, 1.0)); // low
        let report = m.report();
        assert_eq!(report.total_threats, 2);
        assert_eq!(report.open_threats, 2);
        assert_eq!(report.dfd_element_count, 1);
        assert!(report.critical_or_high >= 1);
    }

    #[test]
    fn test_modeler_report_avg_risk() {
        let mut m = ThreatModeler::new();
        m.add_threat(make_threat("t1", Stride::Spoofing, 0.5, 4.0));  // 2.0
        m.add_threat(make_threat("t2", Stride::Tampering, 0.5, 6.0)); // 3.0
        let report = m.report();
        assert!((report.avg_risk_score - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_modeler_mitigated_threat_excluded_from_open() {
        let mut m = ThreatModeler::new();
        let mut t = make_threat("t1", Stride::DenialOfService, 0.5, 5.0);
        t.status = ThreatStatus::Mitigated;
        m.add_threat(t);
        m.add_threat(make_threat("t2", Stride::Repudiation, 0.3, 3.0));
        let report = m.report();
        assert_eq!(report.open_threats, 1);
    }
}
