#[derive(Debug, Clone, PartialEq)]
pub enum ThreatCategory {
    Malware,
    Phishing,
    Exfiltration,
    LateralMovement,
    PrivilegeEscalation,
    C2,
    Ransomware,
    InsiderThreat,
    DDoS,
    SupplyChain,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncidentSeverity {
    P1Critical,
    P2High,
    P3Medium,
    P4Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncidentStatus {
    Open,
    Investigating,
    Contained,
    Eradicated,
    Recovered,
    Closed,
    PostMortem,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SiemPlatform {
    Splunk,
    Sentinel,
    ElasticSIEM,
    QRadar,
    CrowdStrike,
    Wazuh,
    Datadog,
    SumoLogic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForensicArtifactType {
    MemoryDump,
    DiskImage,
    NetworkCapture,
    LogBundle,
    RegistryHive,
    BrowserHistory,
    ProcessList,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IocType {
    IpAddress,
    Domain,
    FileHash,
    Url,
    Email,
    UserAgent,
    CveId,
    RegistryKey,
    Mutex,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybookActionType {
    Isolate,
    Block,
    Notify,
    Collect,
    Scan,
    Remediate,
    Escalate,
    Document,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreatIndicator {
    pub id: String,
    pub ioc_type: IocType,
    pub value: String,
    pub confidence: f64,
    pub source: String,
    pub first_seen: String,
    pub last_seen: String,
    pub tags: Vec<String>,
    pub related_incidents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncidentRecord {
    pub id: String,
    pub title: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub category: ThreatCategory,
    pub timeline: Vec<TimelineEntry>,
    pub affected_assets: Vec<String>,
    pub iocs: Vec<String>,
    pub playbook_id: Option<String>,
    pub assignee: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEntry {
    pub timestamp: String,
    pub action: String,
    pub actor: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectionRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub platform: SiemPlatform,
    pub query: String,
    pub mitre_ids: Vec<String>,
    pub severity: IncidentSeverity,
    pub enabled: bool,
    pub false_positive_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForensicCase {
    pub id: String,
    pub incident_id: String,
    pub artifacts: Vec<ForensicArtifact>,
    pub timeline: Vec<TimelineEntry>,
    pub findings: Vec<String>,
    pub chain_of_custody: Vec<CustodyEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForensicArtifact {
    pub id: String,
    pub artifact_type: ForensicArtifactType,
    pub source: String,
    pub hash: String,
    pub size_bytes: u64,
    pub collected_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustodyEntry {
    pub timestamp: String,
    pub handler: String,
    pub action: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiemConnection {
    pub platform: SiemPlatform,
    pub endpoint_url: String,
    pub api_key_env: String,
    pub index_name: String,
    pub enabled: bool,
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaybookStep {
    pub order: u32,
    pub action_type: PlaybookActionType,
    pub description: String,
    pub automated: bool,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Playbook {
    pub id: String,
    pub name: String,
    pub threat_category: ThreatCategory,
    pub steps: Vec<PlaybookStep>,
    pub last_updated: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreatHuntQuery {
    pub id: String,
    pub name: String,
    pub hypothesis: String,
    pub data_sources: Vec<String>,
    pub query: String,
    pub platform: SiemPlatform,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlueTeamManager {
    pub incidents: Vec<IncidentRecord>,
    pub rules: Vec<DetectionRule>,
    pub iocs: Vec<ThreatIndicator>,
    pub siem_connections: Vec<SiemConnection>,
    pub playbooks: Vec<Playbook>,
    pub forensic_cases: Vec<ForensicCase>,
    pub hunt_queries: Vec<ThreatHuntQuery>,
    next_id: u64,
}

impl BlueTeamManager {
    pub fn new() -> Self {
        Self {
            incidents: Vec::new(),
            rules: Vec::new(),
            iocs: Vec::new(),
            siem_connections: Vec::new(),
            playbooks: Vec::new(),
            forensic_cases: Vec::new(),
            hunt_queries: Vec::new(),
            next_id: 1,
        }
    }

    fn gen_id(&mut self, prefix: &str) -> String {
        let id = format!("{}-{:04}", prefix, self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add_incident(
        &mut self,
        title: &str,
        severity: IncidentSeverity,
        category: ThreatCategory,
    ) -> String {
        let id = self.gen_id("INC");
        let now = "2026-03-14T00:00:00Z".to_string();
        self.incidents.push(IncidentRecord {
            id: id.clone(),
            title: title.to_string(),
            severity,
            status: IncidentStatus::Open,
            category,
            timeline: vec![TimelineEntry {
                timestamp: now.clone(),
                action: "Incident created".to_string(),
                actor: "system".to_string(),
                details: format!("Incident '{}' opened", title),
            }],
            affected_assets: Vec::new(),
            iocs: Vec::new(),
            playbook_id: None,
            assignee: None,
            created_at: now.clone(),
            updated_at: now,
        });
        id
    }

    pub fn update_incident_status(&mut self, id: &str, status: IncidentStatus) -> bool {
        if let Some(inc) = self.incidents.iter_mut().find(|i| i.id == id) {
            inc.status = status;
            inc.updated_at = "2026-03-14T00:00:01Z".to_string();
            true
        } else {
            false
        }
    }

    pub fn add_timeline_entry(
        &mut self,
        incident_id: &str,
        action: &str,
        actor: &str,
        details: &str,
    ) -> bool {
        if let Some(inc) = self.incidents.iter_mut().find(|i| i.id == incident_id) {
            inc.timeline.push(TimelineEntry {
                timestamp: "2026-03-14T00:00:02Z".to_string(),
                action: action.to_string(),
                actor: actor.to_string(),
                details: details.to_string(),
            });
            inc.updated_at = "2026-03-14T00:00:02Z".to_string();
            true
        } else {
            false
        }
    }

    pub fn add_detection_rule(
        &mut self,
        name: &str,
        description: &str,
        platform: SiemPlatform,
        query: &str,
        mitre_ids: Vec<String>,
        severity: IncidentSeverity,
    ) -> String {
        let id = self.gen_id("RULE");
        self.rules.push(DetectionRule {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            platform,
            query: query.to_string(),
            mitre_ids,
            severity,
            enabled: true,
            false_positive_rate: 0.0,
        });
        id
    }

    pub fn toggle_rule(&mut self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = !rule.enabled;
            true
        } else {
            false
        }
    }

    pub fn add_ioc(
        &mut self,
        ioc_type: IocType,
        value: &str,
        confidence: f64,
        source: &str,
    ) -> String {
        let id = self.gen_id("IOC");
        let now = "2026-03-14T00:00:00Z".to_string();
        self.iocs.push(ThreatIndicator {
            id: id.clone(),
            ioc_type,
            value: value.to_string(),
            confidence,
            source: source.to_string(),
            first_seen: now.clone(),
            last_seen: now,
            tags: Vec::new(),
            related_incidents: Vec::new(),
        });
        id
    }

    pub fn search_iocs(&self, query: &str) -> Vec<&ThreatIndicator> {
        let lower = query.to_lowercase();
        self.iocs
            .iter()
            .filter(|ioc| ioc.value.to_lowercase().contains(&lower))
            .collect()
    }

    pub fn search_iocs_by_type(&self, ioc_type: &IocType) -> Vec<&ThreatIndicator> {
        self.iocs
            .iter()
            .filter(|ioc| &ioc.ioc_type == ioc_type)
            .collect()
    }

    pub fn connect_siem(
        &mut self,
        platform: SiemPlatform,
        endpoint_url: &str,
        api_key_env: &str,
        index_name: &str,
    ) -> String {
        let id = format!("SIEM-{}", self.siem_connections.len() + 1);
        self.siem_connections.push(SiemConnection {
            platform,
            endpoint_url: endpoint_url.to_string(),
            api_key_env: api_key_env.to_string(),
            index_name: index_name.to_string(),
            enabled: true,
            last_sync: None,
        });
        id
    }

    pub fn generate_detection_query(&self, platform: &SiemPlatform, ioc: &str) -> String {
        match platform {
            SiemPlatform::Splunk => {
                format!("index=main (src_ip=\"{}\" OR dest_ip=\"{}\" OR url=\"{}\" OR hash=\"{}\")", ioc, ioc, ioc, ioc)
            }
            SiemPlatform::Sentinel => {
                format!(
                    "SecurityEvent | where RemoteIP == \"{}\" or RequestUrl contains \"{}\" or FileHash == \"{}\"",
                    ioc, ioc, ioc
                )
            }
            SiemPlatform::ElasticSIEM => {
                format!(
                    "source.ip:\"{}\" OR destination.ip:\"{}\" OR url.full:\"{}\" OR file.hash.sha256:\"{}\"",
                    ioc, ioc, ioc, ioc
                )
            }
            SiemPlatform::QRadar => {
                format!(
                    "SELECT * FROM events WHERE sourceip='{}' OR destinationip='{}' OR URL LIKE '%{}%'",
                    ioc, ioc, ioc
                )
            }
            SiemPlatform::CrowdStrike => {
                format!(
                    "event_simpleName=* | search RemoteAddressIP4=\"{}\" OR TargetFileName=\"{}\"",
                    ioc, ioc
                )
            }
            SiemPlatform::Wazuh => {
                format!(
                    "rule.groups:\"threat_intel\" AND (data.srcip:\"{}\" OR data.url:\"{}\")",
                    ioc, ioc
                )
            }
            SiemPlatform::Datadog => {
                format!("source:security @network.client.ip:\"{}\" OR @http.url:\"{}\"", ioc, ioc)
            }
            SiemPlatform::SumoLogic => {
                format!(
                    "_sourceCategory=security | where src_ip=\"{}\" or url contains \"{}\"",
                    ioc, ioc
                )
            }
        }
    }

    pub fn create_forensic_case(&mut self, incident_id: &str) -> Option<String> {
        if !self.incidents.iter().any(|i| i.id == incident_id) {
            return None;
        }
        let id = self.gen_id("CASE");
        self.forensic_cases.push(ForensicCase {
            id: id.clone(),
            incident_id: incident_id.to_string(),
            artifacts: Vec::new(),
            timeline: Vec::new(),
            findings: Vec::new(),
            chain_of_custody: Vec::new(),
        });
        Some(id)
    }

    pub fn add_forensic_artifact(
        &mut self,
        case_id: &str,
        artifact_type: ForensicArtifactType,
        source: &str,
        hash: &str,
        size: u64,
    ) -> bool {
        if let Some(case) = self.forensic_cases.iter_mut().find(|c| c.id == case_id) {
            let art_id = format!("{}-ART-{}", case_id, case.artifacts.len() + 1);
            case.artifacts.push(ForensicArtifact {
                id: art_id,
                artifact_type,
                source: source.to_string(),
                hash: hash.to_string(),
                size_bytes: size,
                collected_at: "2026-03-14T00:00:00Z".to_string(),
            });
            true
        } else {
            false
        }
    }

    pub fn add_chain_of_custody(
        &mut self,
        case_id: &str,
        handler: &str,
        action: &str,
        notes: &str,
    ) -> bool {
        if let Some(case) = self.forensic_cases.iter_mut().find(|c| c.id == case_id) {
            case.chain_of_custody.push(CustodyEntry {
                timestamp: "2026-03-14T00:00:00Z".to_string(),
                handler: handler.to_string(),
                action: action.to_string(),
                notes: notes.to_string(),
            });
            true
        } else {
            false
        }
    }

    pub fn add_playbook(&mut self, name: &str, threat_category: ThreatCategory) -> String {
        let id = self.gen_id("PB");
        self.playbooks.push(Playbook {
            id: id.clone(),
            name: name.to_string(),
            threat_category,
            steps: Vec::new(),
            last_updated: "2026-03-14T00:00:00Z".to_string(),
            version: "1.0".to_string(),
        });
        id
    }

    pub fn add_playbook_step(
        &mut self,
        playbook_id: &str,
        action_type: PlaybookActionType,
        description: &str,
        automated: bool,
        timeout: u64,
    ) -> bool {
        if let Some(pb) = self.playbooks.iter_mut().find(|p| p.id == playbook_id) {
            let order = pb.steps.len() as u32 + 1;
            pb.steps.push(PlaybookStep {
                order,
                action_type,
                description: description.to_string(),
                automated,
                timeout_secs: timeout,
            });
            true
        } else {
            false
        }
    }

    pub fn run_playbook_dry(&self, playbook_id: &str) -> Option<Vec<String>> {
        self.playbooks
            .iter()
            .find(|p| p.id == playbook_id)
            .map(|pb| {
                pb.steps
                    .iter()
                    .map(|s| {
                        format!(
                            "Step {}: [{}] {} (automated={}, timeout={}s)",
                            s.order,
                            match &s.action_type {
                                PlaybookActionType::Isolate => "Isolate",
                                PlaybookActionType::Block => "Block",
                                PlaybookActionType::Notify => "Notify",
                                PlaybookActionType::Collect => "Collect",
                                PlaybookActionType::Scan => "Scan",
                                PlaybookActionType::Remediate => "Remediate",
                                PlaybookActionType::Escalate => "Escalate",
                                PlaybookActionType::Document => "Document",
                            },
                            s.description,
                            s.automated,
                            s.timeout_secs
                        )
                    })
                    .collect()
            })
    }

    pub fn add_hunt_query(
        &mut self,
        name: &str,
        hypothesis: &str,
        data_sources: Vec<String>,
        query: &str,
        platform: SiemPlatform,
    ) -> String {
        let id = self.gen_id("HUNT");
        self.hunt_queries.push(ThreatHuntQuery {
            id: id.clone(),
            name: name.to_string(),
            hypothesis: hypothesis.to_string(),
            data_sources,
            query: query.to_string(),
            platform,
            findings: Vec::new(),
        });
        id
    }

    pub fn get_incident(&self, id: &str) -> Option<&IncidentRecord> {
        self.incidents.iter().find(|i| i.id == id)
    }

    pub fn get_open_incidents(&self) -> Vec<&IncidentRecord> {
        self.incidents
            .iter()
            .filter(|i| i.status == IncidentStatus::Open)
            .collect()
    }

    pub fn get_incidents_by_severity(&self, severity: &IncidentSeverity) -> Vec<&IncidentRecord> {
        self.incidents
            .iter()
            .filter(|i| &i.severity == severity)
            .collect()
    }

    pub fn correlate_iocs(&self, incident_id: &str) -> Vec<&ThreatIndicator> {
        self.iocs
            .iter()
            .filter(|ioc| ioc.related_incidents.iter().any(|ri| ri == incident_id))
            .collect()
    }

    pub fn export_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Blue Team Report\n\n");

        report.push_str("## Incidents\n\n");
        if self.incidents.is_empty() {
            report.push_str("No incidents recorded.\n\n");
        } else {
            for inc in &self.incidents {
                report.push_str(&format!(
                    "- **{}** [{}] {:?} | Status: {:?} | Category: {:?}\n",
                    inc.id, inc.title, inc.severity, inc.status, inc.category
                ));
            }
            report.push('\n');
        }

        report.push_str("## IOCs\n\n");
        if self.iocs.is_empty() {
            report.push_str("No IOCs recorded.\n\n");
        } else {
            for ioc in &self.iocs {
                report.push_str(&format!(
                    "- **{}** {:?}: `{}` (confidence: {:.0}%)\n",
                    ioc.id,
                    ioc.ioc_type,
                    ioc.value,
                    ioc.confidence * 100.0
                ));
            }
            report.push('\n');
        }

        report.push_str("## Detection Rules\n\n");
        if self.rules.is_empty() {
            report.push_str("No detection rules configured.\n\n");
        } else {
            for rule in &self.rules {
                report.push_str(&format!(
                    "- **{}** [{}] {:?} | Enabled: {} | Platform: {:?}\n",
                    rule.id, rule.name, rule.severity, rule.enabled, rule.platform
                ));
            }
            report.push('\n');
        }

        report.push_str(&format!("## Summary\n\n"));
        report.push_str(&format!("- Total incidents: {}\n", self.incidents.len()));
        report.push_str(&format!(
            "- Open incidents: {}\n",
            self.incidents
                .iter()
                .filter(|i| i.status == IncidentStatus::Open)
                .count()
        ));
        report.push_str(&format!("- Total IOCs: {}\n", self.iocs.len()));
        report.push_str(&format!("- Detection rules: {}\n", self.rules.len()));
        report.push_str(&format!("- Playbooks: {}\n", self.playbooks.len()));
        report.push_str(&format!("- Forensic cases: {}\n", self.forensic_cases.len()));
        report.push_str(&format!("- Hunt queries: {}\n", self.hunt_queries.len()));

        report
    }

    pub fn calculate_mttr(&self) -> Option<f64> {
        // Calculate mean time to respond based on incidents that have at least 2 timeline entries
        // (creation + first response action)
        let response_times: Vec<f64> = self
            .incidents
            .iter()
            .filter(|i| i.timeline.len() >= 2)
            .map(|i| {
                // Simplified: use timeline entry count as a proxy for response time in minutes
                // In production, this would parse actual timestamps
                (i.timeline.len() - 1) as f64 * 15.0 // 15 minutes per action as approximation
            })
            .collect();

        if response_times.is_empty() {
            return None;
        }

        let sum: f64 = response_times.iter().sum();
        Some(sum / response_times.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> BlueTeamManager {
        BlueTeamManager::new()
    }

    #[test]
    fn test_new_manager_empty() {
        let mgr = make_manager();
        assert!(mgr.incidents.is_empty());
        assert!(mgr.rules.is_empty());
        assert!(mgr.iocs.is_empty());
        assert!(mgr.siem_connections.is_empty());
        assert!(mgr.playbooks.is_empty());
        assert!(mgr.forensic_cases.is_empty());
        assert!(mgr.hunt_queries.is_empty());
    }

    #[test]
    fn test_add_incident() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Phishing campaign", IncidentSeverity::P2High, ThreatCategory::Phishing);
        assert!(id.starts_with("INC-"));
        assert_eq!(mgr.incidents.len(), 1);
        assert_eq!(mgr.incidents[0].title, "Phishing campaign");
        assert_eq!(mgr.incidents[0].status, IncidentStatus::Open);
    }

    #[test]
    fn test_add_multiple_incidents() {
        let mut mgr = make_manager();
        let id1 = mgr.add_incident("Inc 1", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        let id2 = mgr.add_incident("Inc 2", IncidentSeverity::P4Low, ThreatCategory::DDoS);
        assert_ne!(id1, id2);
        assert_eq!(mgr.incidents.len(), 2);
    }

    #[test]
    fn test_update_incident_status() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Test", IncidentSeverity::P3Medium, ThreatCategory::C2);
        assert!(mgr.update_incident_status(&id, IncidentStatus::Investigating));
        assert_eq!(mgr.incidents[0].status, IncidentStatus::Investigating);
    }

    #[test]
    fn test_update_incident_status_unknown_id() {
        let mut mgr = make_manager();
        assert!(!mgr.update_incident_status("FAKE-999", IncidentStatus::Closed));
    }

    #[test]
    fn test_add_timeline_entry() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Test", IncidentSeverity::P1Critical, ThreatCategory::Ransomware);
        assert!(mgr.add_timeline_entry(&id, "Contained host", "analyst1", "Isolated server-01"));
        assert_eq!(mgr.incidents[0].timeline.len(), 2); // creation + new entry
    }

    #[test]
    fn test_add_timeline_entry_unknown_incident() {
        let mut mgr = make_manager();
        assert!(!mgr.add_timeline_entry("NOPE", "action", "actor", "details"));
    }

    #[test]
    fn test_add_detection_rule() {
        let mut mgr = make_manager();
        let id = mgr.add_detection_rule(
            "SSH Brute Force",
            "Detect SSH brute force attempts",
            SiemPlatform::Splunk,
            "index=auth action=failure | stats count by src_ip",
            vec!["T1110".to_string()],
            IncidentSeverity::P2High,
        );
        assert!(id.starts_with("RULE-"));
        assert_eq!(mgr.rules.len(), 1);
        assert!(mgr.rules[0].enabled);
    }

    #[test]
    fn test_toggle_rule() {
        let mut mgr = make_manager();
        let id = mgr.add_detection_rule(
            "Test Rule", "desc", SiemPlatform::Sentinel, "query", vec![], IncidentSeverity::P4Low,
        );
        assert!(mgr.rules[0].enabled);
        assert!(mgr.toggle_rule(&id));
        assert!(!mgr.rules[0].enabled);
        assert!(mgr.toggle_rule(&id));
        assert!(mgr.rules[0].enabled);
    }

    #[test]
    fn test_toggle_rule_unknown() {
        let mut mgr = make_manager();
        assert!(!mgr.toggle_rule("RULE-FAKE"));
    }

    #[test]
    fn test_add_ioc() {
        let mut mgr = make_manager();
        let id = mgr.add_ioc(IocType::IpAddress, "192.168.1.100", 0.95, "threat-intel-feed");
        assert!(id.starts_with("IOC-"));
        assert_eq!(mgr.iocs.len(), 1);
        assert_eq!(mgr.iocs[0].confidence, 0.95);
    }

    #[test]
    fn test_search_iocs_by_value() {
        let mut mgr = make_manager();
        mgr.add_ioc(IocType::Domain, "evil.example.com", 0.9, "feed1");
        mgr.add_ioc(IocType::Domain, "good.example.org", 0.1, "feed2");
        mgr.add_ioc(IocType::IpAddress, "10.0.0.1", 0.5, "feed3");

        let results = mgr.search_iocs("example");
        assert_eq!(results.len(), 2);

        let results = mgr.search_iocs("evil");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "evil.example.com");
    }

    #[test]
    fn test_search_iocs_case_insensitive() {
        let mut mgr = make_manager();
        mgr.add_ioc(IocType::Domain, "Evil.Example.COM", 0.9, "feed");
        let results = mgr.search_iocs("evil");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_iocs_empty() {
        let mgr = make_manager();
        assert!(mgr.search_iocs("anything").is_empty());
    }

    #[test]
    fn test_search_iocs_by_type() {
        let mut mgr = make_manager();
        mgr.add_ioc(IocType::IpAddress, "10.0.0.1", 0.8, "s1");
        mgr.add_ioc(IocType::Domain, "bad.com", 0.7, "s2");
        mgr.add_ioc(IocType::IpAddress, "10.0.0.2", 0.6, "s3");

        let ips = mgr.search_iocs_by_type(&IocType::IpAddress);
        assert_eq!(ips.len(), 2);

        let domains = mgr.search_iocs_by_type(&IocType::Domain);
        assert_eq!(domains.len(), 1);

        let hashes = mgr.search_iocs_by_type(&IocType::FileHash);
        assert!(hashes.is_empty());
    }

    #[test]
    fn test_connect_siem() {
        let mut mgr = make_manager();
        let id = mgr.connect_siem(
            SiemPlatform::Splunk,
            "https://splunk.corp.com:8089",
            "SPLUNK_API_KEY",
            "main",
        );
        assert!(id.starts_with("SIEM-"));
        assert_eq!(mgr.siem_connections.len(), 1);
        assert!(mgr.siem_connections[0].enabled);
        assert!(mgr.siem_connections[0].last_sync.is_none());
    }

    #[test]
    fn test_generate_detection_query_splunk() {
        let mgr = make_manager();
        let q = mgr.generate_detection_query(&SiemPlatform::Splunk, "10.0.0.1");
        assert!(q.contains("index=main"));
        assert!(q.contains("10.0.0.1"));
    }

    #[test]
    fn test_generate_detection_query_sentinel() {
        let mgr = make_manager();
        let q = mgr.generate_detection_query(&SiemPlatform::Sentinel, "bad.com");
        assert!(q.contains("SecurityEvent"));
        assert!(q.contains("bad.com"));
    }

    #[test]
    fn test_generate_detection_query_elastic() {
        let mgr = make_manager();
        let q = mgr.generate_detection_query(&SiemPlatform::ElasticSIEM, "hash123");
        assert!(q.contains("source.ip"));
        assert!(q.contains("hash123"));
    }

    #[test]
    fn test_generate_detection_query_qradar() {
        let mgr = make_manager();
        let q = mgr.generate_detection_query(&SiemPlatform::QRadar, "1.2.3.4");
        assert!(q.contains("SELECT"));
        assert!(q.contains("1.2.3.4"));
    }

    #[test]
    fn test_generate_detection_query_all_platforms() {
        let mgr = make_manager();
        let platforms = vec![
            SiemPlatform::Splunk,
            SiemPlatform::Sentinel,
            SiemPlatform::ElasticSIEM,
            SiemPlatform::QRadar,
            SiemPlatform::CrowdStrike,
            SiemPlatform::Wazuh,
            SiemPlatform::Datadog,
            SiemPlatform::SumoLogic,
        ];
        for p in platforms {
            let q = mgr.generate_detection_query(&p, "test-ioc");
            assert!(!q.is_empty());
            assert!(q.contains("test-ioc"));
        }
    }

    #[test]
    fn test_create_forensic_case() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Test", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        let case_id = mgr.create_forensic_case(&inc_id);
        assert!(case_id.is_some());
        assert!(case_id.unwrap().starts_with("CASE-"));
        assert_eq!(mgr.forensic_cases.len(), 1);
    }

    #[test]
    fn test_create_forensic_case_unknown_incident() {
        let mut mgr = make_manager();
        assert!(mgr.create_forensic_case("NOPE").is_none());
    }

    #[test]
    fn test_add_forensic_artifact() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Test", IncidentSeverity::P2High, ThreatCategory::Exfiltration);
        let case_id = mgr.create_forensic_case(&inc_id).expect("case created");
        assert!(mgr.add_forensic_artifact(
            &case_id,
            ForensicArtifactType::MemoryDump,
            "server-01",
            "sha256:abc123",
            1024000,
        ));
        assert_eq!(mgr.forensic_cases[0].artifacts.len(), 1);
        assert_eq!(mgr.forensic_cases[0].artifacts[0].size_bytes, 1024000);
    }

    #[test]
    fn test_add_forensic_artifact_unknown_case() {
        let mut mgr = make_manager();
        assert!(!mgr.add_forensic_artifact(
            "CASE-FAKE",
            ForensicArtifactType::DiskImage,
            "src",
            "hash",
            100,
        ));
    }

    #[test]
    fn test_add_chain_of_custody() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Test", IncidentSeverity::P3Medium, ThreatCategory::InsiderThreat);
        let case_id = mgr.create_forensic_case(&inc_id).expect("case");
        assert!(mgr.add_chain_of_custody(&case_id, "analyst1", "Collected", "RAM dump from workstation"));
        assert!(mgr.add_chain_of_custody(&case_id, "forensics-lab", "Received", "Verified hash"));
        assert_eq!(mgr.forensic_cases[0].chain_of_custody.len(), 2);
        assert_eq!(mgr.forensic_cases[0].chain_of_custody[0].handler, "analyst1");
        assert_eq!(mgr.forensic_cases[0].chain_of_custody[1].handler, "forensics-lab");
    }

    #[test]
    fn test_add_chain_of_custody_unknown_case() {
        let mut mgr = make_manager();
        assert!(!mgr.add_chain_of_custody("FAKE", "h", "a", "n"));
    }

    #[test]
    fn test_add_playbook() {
        let mut mgr = make_manager();
        let id = mgr.add_playbook("Ransomware Response", ThreatCategory::Ransomware);
        assert!(id.starts_with("PB-"));
        assert_eq!(mgr.playbooks.len(), 1);
        assert_eq!(mgr.playbooks[0].version, "1.0");
    }

    #[test]
    fn test_add_playbook_steps() {
        let mut mgr = make_manager();
        let pb_id = mgr.add_playbook("Phishing Response", ThreatCategory::Phishing);
        assert!(mgr.add_playbook_step(&pb_id, PlaybookActionType::Isolate, "Isolate affected endpoint", true, 60));
        assert!(mgr.add_playbook_step(&pb_id, PlaybookActionType::Collect, "Collect email headers", false, 300));
        assert!(mgr.add_playbook_step(&pb_id, PlaybookActionType::Block, "Block sender domain", true, 30));
        assert_eq!(mgr.playbooks[0].steps.len(), 3);
        assert_eq!(mgr.playbooks[0].steps[0].order, 1);
        assert_eq!(mgr.playbooks[0].steps[1].order, 2);
        assert_eq!(mgr.playbooks[0].steps[2].order, 3);
    }

    #[test]
    fn test_add_playbook_step_unknown() {
        let mut mgr = make_manager();
        assert!(!mgr.add_playbook_step("PB-FAKE", PlaybookActionType::Notify, "desc", false, 10));
    }

    #[test]
    fn test_run_playbook_dry() {
        let mut mgr = make_manager();
        let pb_id = mgr.add_playbook("IR Plan", ThreatCategory::Malware);
        mgr.add_playbook_step(&pb_id, PlaybookActionType::Isolate, "Isolate host", true, 60);
        mgr.add_playbook_step(&pb_id, PlaybookActionType::Scan, "Run AV scan", true, 600);

        let result = mgr.run_playbook_dry(&pb_id);
        assert!(result.is_some());
        let steps = result.unwrap();
        assert_eq!(steps.len(), 2);
        assert!(steps[0].contains("Step 1"));
        assert!(steps[0].contains("Isolate"));
        assert!(steps[1].contains("Step 2"));
        assert!(steps[1].contains("Scan"));
    }

    #[test]
    fn test_run_playbook_dry_unknown() {
        let mgr = make_manager();
        assert!(mgr.run_playbook_dry("PB-NOPE").is_none());
    }

    #[test]
    fn test_run_playbook_dry_empty_steps() {
        let mut mgr = make_manager();
        let pb_id = mgr.add_playbook("Empty", ThreatCategory::DDoS);
        let result = mgr.run_playbook_dry(&pb_id);
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_add_hunt_query() {
        let mut mgr = make_manager();
        let id = mgr.add_hunt_query(
            "Lateral Movement Hunt",
            "Attackers may use RDP for lateral movement",
            vec!["windows_event_log".to_string(), "netflow".to_string()],
            "EventID=4624 LogonType=10",
            SiemPlatform::Splunk,
        );
        assert!(id.starts_with("HUNT-"));
        assert_eq!(mgr.hunt_queries.len(), 1);
        assert_eq!(mgr.hunt_queries[0].data_sources.len(), 2);
    }

    #[test]
    fn test_get_incident() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Found it", IncidentSeverity::P4Low, ThreatCategory::SupplyChain);
        assert!(mgr.get_incident(&id).is_some());
        assert_eq!(mgr.get_incident(&id).unwrap().title, "Found it");
        assert!(mgr.get_incident("NONEXISTENT").is_none());
    }

    #[test]
    fn test_get_open_incidents() {
        let mut mgr = make_manager();
        let id1 = mgr.add_incident("Open 1", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        mgr.add_incident("Open 2", IncidentSeverity::P2High, ThreatCategory::Phishing);
        mgr.update_incident_status(&id1, IncidentStatus::Closed);

        let open = mgr.get_open_incidents();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].title, "Open 2");
    }

    #[test]
    fn test_get_incidents_by_severity() {
        let mut mgr = make_manager();
        mgr.add_incident("Crit 1", IncidentSeverity::P1Critical, ThreatCategory::Ransomware);
        mgr.add_incident("Low 1", IncidentSeverity::P4Low, ThreatCategory::DDoS);
        mgr.add_incident("Crit 2", IncidentSeverity::P1Critical, ThreatCategory::C2);

        let crits = mgr.get_incidents_by_severity(&IncidentSeverity::P1Critical);
        assert_eq!(crits.len(), 2);

        let lows = mgr.get_incidents_by_severity(&IncidentSeverity::P4Low);
        assert_eq!(lows.len(), 1);

        let meds = mgr.get_incidents_by_severity(&IncidentSeverity::P3Medium);
        assert!(meds.is_empty());
    }

    #[test]
    fn test_correlate_iocs() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Test", IncidentSeverity::P2High, ThreatCategory::Exfiltration);
        mgr.add_ioc(IocType::IpAddress, "10.0.0.1", 0.9, "feed");
        mgr.add_ioc(IocType::Domain, "evil.com", 0.8, "feed");

        // Link first IOC to incident
        mgr.iocs[0].related_incidents.push(inc_id.clone());

        let correlated = mgr.correlate_iocs(&inc_id);
        assert_eq!(correlated.len(), 1);
        assert_eq!(correlated[0].value, "10.0.0.1");
    }

    #[test]
    fn test_correlate_iocs_none() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Test", IncidentSeverity::P4Low, ThreatCategory::Phishing);
        assert!(mgr.correlate_iocs(&inc_id).is_empty());
    }

    #[test]
    fn test_export_report_empty() {
        let mgr = make_manager();
        let report = mgr.export_report();
        assert!(report.contains("# Blue Team Report"));
        assert!(report.contains("No incidents recorded."));
        assert!(report.contains("No IOCs recorded."));
        assert!(report.contains("Total incidents: 0"));
    }

    #[test]
    fn test_export_report_with_data() {
        let mut mgr = make_manager();
        mgr.add_incident("Breach", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        mgr.add_ioc(IocType::FileHash, "abc123def", 0.95, "virustotal");
        mgr.add_detection_rule("Rule1", "desc", SiemPlatform::Splunk, "q", vec![], IncidentSeverity::P2High);

        let report = mgr.export_report();
        assert!(report.contains("Breach"));
        assert!(report.contains("abc123def"));
        assert!(report.contains("95%"));
        assert!(report.contains("Rule1"));
        assert!(report.contains("Total incidents: 1"));
        assert!(report.contains("Total IOCs: 1"));
        assert!(report.contains("Detection rules: 1"));
    }

    #[test]
    fn test_calculate_mttr_no_incidents() {
        let mgr = make_manager();
        assert!(mgr.calculate_mttr().is_none());
    }

    #[test]
    fn test_calculate_mttr_single_entry_only() {
        let mut mgr = make_manager();
        // Incident with only creation entry (1 timeline entry) should not count
        mgr.add_incident("Test", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        assert!(mgr.calculate_mttr().is_none());
    }

    #[test]
    fn test_calculate_mttr_with_responses() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Test", IncidentSeverity::P1Critical, ThreatCategory::Ransomware);
        mgr.add_timeline_entry(&id, "Responded", "analyst", "First response");
        // 2 entries total -> (2-1) * 15 = 15 minutes MTTR
        let mttr = mgr.calculate_mttr();
        assert!(mttr.is_some());
        assert!((mttr.unwrap() - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_incident_created_with_timeline() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Auto-timeline", IncidentSeverity::P3Medium, ThreatCategory::LateralMovement);
        let inc = mgr.get_incident(&id).unwrap();
        assert_eq!(inc.timeline.len(), 1);
        assert_eq!(inc.timeline[0].action, "Incident created");
        assert_eq!(inc.timeline[0].actor, "system");
    }

    #[test]
    fn test_incident_status_transitions() {
        let mut mgr = make_manager();
        let id = mgr.add_incident("Lifecycle", IncidentSeverity::P2High, ThreatCategory::PrivilegeEscalation);

        let statuses = vec![
            IncidentStatus::Investigating,
            IncidentStatus::Contained,
            IncidentStatus::Eradicated,
            IncidentStatus::Recovered,
            IncidentStatus::Closed,
            IncidentStatus::PostMortem,
        ];
        for status in statuses {
            let expected = status.clone();
            assert!(mgr.update_incident_status(&id, status));
            assert_eq!(mgr.get_incident(&id).unwrap().status, expected);
        }
    }

    #[test]
    fn test_forensic_multiple_artifacts() {
        let mut mgr = make_manager();
        let inc_id = mgr.add_incident("Multi-art", IncidentSeverity::P1Critical, ThreatCategory::Malware);
        let case_id = mgr.create_forensic_case(&inc_id).unwrap();

        let artifact_types = vec![
            ForensicArtifactType::MemoryDump,
            ForensicArtifactType::DiskImage,
            ForensicArtifactType::NetworkCapture,
            ForensicArtifactType::LogBundle,
            ForensicArtifactType::RegistryHive,
            ForensicArtifactType::BrowserHistory,
            ForensicArtifactType::ProcessList,
        ];
        for art_type in &artifact_types {
            assert!(mgr.add_forensic_artifact(&case_id, art_type.clone(), "src", "hash", 100));
        }
        assert_eq!(mgr.forensic_cases[0].artifacts.len(), 7);
    }

    #[test]
    fn test_unique_ids() {
        let mut mgr = make_manager();
        let id1 = mgr.add_incident("A", IncidentSeverity::P4Low, ThreatCategory::DDoS);
        let id2 = mgr.add_ioc(IocType::Domain, "x.com", 0.5, "s");
        let id3 = mgr.add_detection_rule("R", "d", SiemPlatform::Wazuh, "q", vec![], IncidentSeverity::P4Low);
        let id4 = mgr.add_playbook("P", ThreatCategory::Phishing);
        // All IDs should be unique
        let ids = vec![&id1, &id2, &id3, &id4];
        for (i, a) in ids.iter().enumerate() {
            for (j, b) in ids.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_threat_category_coverage() {
        let mut mgr = make_manager();
        let categories = vec![
            ThreatCategory::Malware,
            ThreatCategory::Phishing,
            ThreatCategory::Exfiltration,
            ThreatCategory::LateralMovement,
            ThreatCategory::PrivilegeEscalation,
            ThreatCategory::C2,
            ThreatCategory::Ransomware,
            ThreatCategory::InsiderThreat,
            ThreatCategory::DDoS,
            ThreatCategory::SupplyChain,
        ];
        for cat in categories {
            let expected = cat.clone();
            mgr.add_incident("test", IncidentSeverity::P3Medium, cat);
            assert_eq!(mgr.incidents.last().unwrap().category, expected);
        }
        assert_eq!(mgr.incidents.len(), 10);
    }
}
