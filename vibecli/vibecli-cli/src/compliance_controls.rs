//! SOC 2 technical controls for enterprise readiness.
//!
//! Implements a compliance control inventory, audit logging, PII redaction,
//! data retention policies, and reporting aligned with SOC 2 Trust Service Criteria.

#[derive(Debug, Clone, PartialEq)]
pub enum TrustServiceCriteria {
    Security,
    Availability,
    ProcessingIntegrity,
    Confidentiality,
    Privacy,
}

impl TrustServiceCriteria {
    pub fn label(&self) -> &str {
        match self {
            TrustServiceCriteria::Security => "Security",
            TrustServiceCriteria::Availability => "Availability",
            TrustServiceCriteria::ProcessingIntegrity => "Processing Integrity",
            TrustServiceCriteria::Confidentiality => "Confidentiality",
            TrustServiceCriteria::Privacy => "Privacy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlStatus {
    Implemented,
    PartiallyImplemented,
    NotImplemented,
    NotApplicable,
}

impl ControlStatus {
    pub fn label(&self) -> &str {
        match self {
            ControlStatus::Implemented => "Implemented",
            ControlStatus::PartiallyImplemented => "Partially Implemented",
            ControlStatus::NotImplemented => "Not Implemented",
            ControlStatus::NotApplicable => "Not Applicable",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComplianceControl {
    pub id: String,
    pub criteria: TrustServiceCriteria,
    pub title: String,
    pub description: String,
    pub status: ControlStatus,
    pub evidence: Vec<String>,
    pub last_assessed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditAction {
    CodeGenerated,
    FileModified,
    FileDeleted,
    AgentStarted,
    AgentCompleted,
    ConfigChanged,
    UserLogin,
    UserLogout,
    ApiKeyRotated,
    BudgetModified,
}

impl AuditAction {
    pub fn label(&self) -> &str {
        match self {
            AuditAction::CodeGenerated => "code_generated",
            AuditAction::FileModified => "file_modified",
            AuditAction::FileDeleted => "file_deleted",
            AuditAction::AgentStarted => "agent_started",
            AuditAction::AgentCompleted => "agent_completed",
            AuditAction::ConfigChanged => "config_changed",
            AuditAction::UserLogin => "user_login",
            AuditAction::UserLogout => "user_logout",
            AuditAction::ApiKeyRotated => "api_key_rotated",
            AuditAction::BudgetModified => "budget_modified",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: u64,
    pub actor: String,
    pub action: AuditAction,
    pub resource: String,
    pub details: String,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataRetentionPolicy {
    pub resource_type: String,
    pub retention_days: u32,
    pub pii_redaction: bool,
    pub archive_after_days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComplianceReport {
    pub generated_at: u64,
    pub controls: Vec<ComplianceControl>,
    pub total_controls: usize,
    pub implemented: usize,
    pub partial: usize,
    pub not_implemented: usize,
    pub score_percent: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PiiType {
    Email,
    Name,
    ApiKey,
    IpAddress,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedactionMethod {
    Hash,
    Mask,
    Remove,
    Tokenize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PiiField {
    pub field_name: String,
    pub field_type: PiiType,
    pub redaction_method: RedactionMethod,
}

#[derive(Debug, Clone)]
pub struct ControlInventory {
    pub controls: Vec<ComplianceControl>,
    pub audit_log: Vec<AuditLogEntry>,
    pub retention_policies: Vec<DataRetentionPolicy>,
}

impl ControlInventory {
    pub fn new() -> Self {
        Self {
            controls: Vec::new(),
            audit_log: Vec::new(),
            retention_policies: Vec::new(),
        }
    }

    pub fn add_control(&mut self, control: ComplianceControl) {
        self.controls.push(control);
    }

    pub fn add_default_soc2_controls(&mut self) {
        let defaults = vec![
            ("CC1.1", TrustServiceCriteria::Security, "Access Control", "Logical and physical access controls to protect information assets"),
            ("CC1.2", TrustServiceCriteria::Security, "Authentication", "Multi-factor authentication for system access"),
            ("CC1.3", TrustServiceCriteria::Security, "Encryption at Rest", "Data encryption at rest using AES-256 or equivalent"),
            ("CC1.4", TrustServiceCriteria::Security, "Encryption in Transit", "TLS 1.2+ for all data in transit"),
            ("CC1.5", TrustServiceCriteria::Security, "Vulnerability Management", "Regular vulnerability scanning and patching"),
            ("CC2.1", TrustServiceCriteria::Availability, "System Monitoring", "Continuous monitoring of system availability and performance"),
            ("CC2.2", TrustServiceCriteria::Availability, "Incident Response", "Documented incident response procedures"),
            ("CC2.3", TrustServiceCriteria::Availability, "Backup and Recovery", "Regular backups with tested recovery procedures"),
            ("CC3.1", TrustServiceCriteria::ProcessingIntegrity, "Input Validation", "Validation of all inputs to prevent injection attacks"),
            ("CC3.2", TrustServiceCriteria::ProcessingIntegrity, "Audit Logging", "Comprehensive audit logging of all system actions"),
            ("CC4.1", TrustServiceCriteria::Confidentiality, "Data Classification", "Classification and handling of confidential data"),
            ("CC4.2", TrustServiceCriteria::Confidentiality, "Key Management", "Secure key management and rotation procedures"),
            ("CC5.1", TrustServiceCriteria::Privacy, "Data Retention", "Defined data retention and disposal policies"),
            ("CC5.2", TrustServiceCriteria::Privacy, "PII Protection", "Identification and protection of personally identifiable information"),
            ("CC5.3", TrustServiceCriteria::Privacy, "Consent Management", "User consent tracking and management"),
        ];

        for (id, criteria, title, desc) in defaults {
            self.controls.push(ComplianceControl {
                id: id.to_string(),
                criteria,
                title: title.to_string(),
                description: desc.to_string(),
                status: ControlStatus::NotImplemented,
                evidence: Vec::new(),
                last_assessed: 0,
            });
        }
    }

    pub fn log_action(&mut self, entry: AuditLogEntry) {
        self.audit_log.push(entry);
    }

    pub fn set_retention_policy(&mut self, policy: DataRetentionPolicy) {
        // Replace existing policy for same resource type, or add new
        if let Some(existing) = self
            .retention_policies
            .iter_mut()
            .find(|p| p.resource_type == policy.resource_type)
        {
            *existing = policy;
        } else {
            self.retention_policies.push(policy);
        }
    }

    pub fn assess_control(
        &mut self,
        id: &str,
        status: ControlStatus,
        evidence: Vec<String>,
    ) -> Result<(), String> {
        let control = self
            .controls
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("Control not found: {}", id))?;
        control.status = status;
        control.evidence = evidence;
        control.last_assessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(())
    }

    pub fn generate_report(&self) -> ComplianceReport {
        let total = self.controls.len();
        let implemented = self
            .controls
            .iter()
            .filter(|c| c.status == ControlStatus::Implemented)
            .count();
        let partial = self
            .controls
            .iter()
            .filter(|c| c.status == ControlStatus::PartiallyImplemented)
            .count();
        let not_implemented = self
            .controls
            .iter()
            .filter(|c| c.status == ControlStatus::NotImplemented)
            .count();

        let applicable = total
            - self
                .controls
                .iter()
                .filter(|c| c.status == ControlStatus::NotApplicable)
                .count();

        let score = if applicable > 0 {
            let weighted = implemented as f64 + (partial as f64 * 0.5);
            (weighted / applicable as f64) * 100.0
        } else {
            0.0
        };

        ComplianceReport {
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            controls: self.controls.clone(),
            total_controls: total,
            implemented,
            partial,
            not_implemented,
            score_percent: score,
        }
    }

    pub fn export_report_markdown(report: &ComplianceReport) -> String {
        let mut md = String::with_capacity(2048);
        md.push_str("# SOC 2 Compliance Report\n\n");
        md.push_str(&format!(
            "- **Generated:** {}\n",
            report.generated_at
        ));
        md.push_str(&format!(
            "- **Total Controls:** {}\n",
            report.total_controls
        ));
        md.push_str(&format!("- **Implemented:** {}\n", report.implemented));
        md.push_str(&format!(
            "- **Partially Implemented:** {}\n",
            report.partial
        ));
        md.push_str(&format!(
            "- **Not Implemented:** {}\n",
            report.not_implemented
        ));
        md.push_str(&format!(
            "- **Compliance Score:** {:.1}%\n\n",
            report.score_percent
        ));

        md.push_str("## Controls\n\n");
        md.push_str("| ID | Criteria | Title | Status |\n");
        md.push_str("|----|----------|-------|--------|\n");
        for c in &report.controls {
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                c.id,
                c.criteria.label(),
                c.title,
                c.status.label()
            ));
        }
        md
    }

    pub fn get_audit_log(&self, start: u64, end: u64) -> Vec<&AuditLogEntry> {
        self.audit_log
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }

    pub fn redact_pii(text: &str, fields: &[PiiField]) -> String {
        let mut result = text.to_string();
        for field in fields {
            // Simple pattern-based redaction
            match &field.field_type {
                PiiType::Email => {
                    // Match email-like patterns
                    let mut output = String::new();
                    let mut remaining = result.as_str();
                    while let Some(at_pos) = remaining.find('@') {
                        // Find start of email (scan backwards for non-space)
                        let before = &remaining[..at_pos];
                        let email_start = before
                            .rfind(|c: char| c.is_whitespace() || c == '<' || c == '(' || c == ',')
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        // Find end of email (scan forward for non-email char)
                        let after = &remaining[at_pos + 1..];
                        let email_end = after
                            .find(|c: char| {
                                c.is_whitespace() || c == '>' || c == ')' || c == ','
                            })
                            .unwrap_or(after.len());
                        let full_end = at_pos + 1 + email_end;
                        let email = &remaining[email_start..full_end];
                        if email.contains('.') && email.len() > 3 {
                            output.push_str(&remaining[..email_start]);
                            output.push_str(&Self::apply_redaction(email, &field.redaction_method));
                            remaining = &remaining[full_end..];
                        } else {
                            output.push_str(&remaining[..full_end]);
                            remaining = &remaining[full_end..];
                        }
                    }
                    output.push_str(remaining);
                    result = output;
                }
                PiiType::ApiKey => {
                    // Redact strings that look like API keys (long alphanumeric sequences)
                    let words: Vec<&str> = result.split_whitespace().collect();
                    let redacted: Vec<String> = words
                        .iter()
                        .map(|w| {
                            if w.len() >= 20
                                && w.chars()
                                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                            {
                                Self::apply_redaction(w, &field.redaction_method)
                            } else {
                                w.to_string()
                            }
                        })
                        .collect();
                    result = redacted.join(" ");
                }
                PiiType::IpAddress => {
                    // Simple IPv4 pattern redaction
                    let mut output = String::new();
                    let mut chars = result.chars().peekable();
                    let mut buf = String::new();
                    while let Some(c) = chars.next() {
                        if c.is_ascii_digit() || c == '.' {
                            buf.push(c);
                        } else {
                            if Self::is_ipv4(&buf) {
                                output.push_str(&Self::apply_redaction(&buf, &field.redaction_method));
                            } else {
                                output.push_str(&buf);
                            }
                            buf.clear();
                            output.push(c);
                        }
                    }
                    if Self::is_ipv4(&buf) {
                        output.push_str(&Self::apply_redaction(&buf, &field.redaction_method));
                    } else {
                        output.push_str(&buf);
                    }
                    result = output;
                }
                PiiType::Name | PiiType::Custom(_) => {
                    // For Name/Custom, redact occurrences of field_name value directly
                    let replacement = Self::apply_redaction(&field.field_name, &field.redaction_method);
                    result = result.replace(&field.field_name, &replacement);
                }
            }
        }
        result
    }

    pub fn purge_expired_logs(&mut self, now: u64) -> usize {
        // Find matching retention policy for audit_log entries
        let default_retention_days = 365u32;
        let retention_secs = self
            .retention_policies
            .iter()
            .find(|p| p.resource_type == "audit_log")
            .map(|p| p.retention_days as u64 * 86400)
            .unwrap_or(default_retention_days as u64 * 86400);

        let cutoff = now.saturating_sub(retention_secs);
        let before = self.audit_log.len();
        self.audit_log.retain(|e| e.timestamp >= cutoff);
        before - self.audit_log.len()
    }

    pub fn compliance_score(&self) -> f64 {
        let report = self.generate_report();
        report.score_percent
    }

    fn apply_redaction(text: &str, method: &RedactionMethod) -> String {
        match method {
            RedactionMethod::Hash => {
                // Simple hash representation
                let hash: u64 = text.bytes().fold(0u64, |acc, b| {
                    acc.wrapping_mul(31).wrapping_add(b as u64)
                });
                format!("[HASH:{:016x}]", hash)
            }
            RedactionMethod::Mask => {
                if text.len() <= 4 {
                    "****".to_string()
                } else {
                    let visible = &text[..2];
                    let masked = "*".repeat(text.len() - 2);
                    format!("{}{}", visible, masked)
                }
            }
            RedactionMethod::Remove => "[REDACTED]".to_string(),
            RedactionMethod::Tokenize => {
                let token: u32 = text
                    .bytes()
                    .fold(0u32, |acc, b| acc.wrapping_mul(7).wrapping_add(b as u32));
                format!("[TOKEN:{}]", token)
            }
        }
    }

    fn is_ipv4(s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.parse::<u8>().is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_inventory_is_empty() {
        let inv = ControlInventory::new();
        assert!(inv.controls.is_empty());
        assert!(inv.audit_log.is_empty());
        assert!(inv.retention_policies.is_empty());
    }

    #[test]
    fn test_add_control() {
        let mut inv = ControlInventory::new();
        inv.add_control(ComplianceControl {
            id: "CC1".to_string(),
            criteria: TrustServiceCriteria::Security,
            title: "Test".to_string(),
            description: "Test control".to_string(),
            status: ControlStatus::NotImplemented,
            evidence: Vec::new(),
            last_assessed: 0,
        });
        assert_eq!(inv.controls.len(), 1);
    }

    #[test]
    fn test_add_default_soc2_controls() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        assert_eq!(inv.controls.len(), 15);
        assert!(inv.controls.iter().any(|c| c.id == "CC1.1"));
        assert!(inv.controls.iter().any(|c| c.id == "CC5.3"));
    }

    #[test]
    fn test_log_action() {
        let mut inv = ControlInventory::new();
        inv.log_action(AuditLogEntry {
            id: "log-1".to_string(),
            timestamp: 1000,
            actor: "user@test.com".to_string(),
            action: AuditAction::CodeGenerated,
            resource: "main.rs".to_string(),
            details: "Generated function".to_string(),
            ip_address: Some("10.0.0.1".to_string()),
        });
        assert_eq!(inv.audit_log.len(), 1);
    }

    #[test]
    fn test_set_retention_policy_new() {
        let mut inv = ControlInventory::new();
        inv.set_retention_policy(DataRetentionPolicy {
            resource_type: "audit_log".to_string(),
            retention_days: 90,
            pii_redaction: true,
            archive_after_days: Some(30),
        });
        assert_eq!(inv.retention_policies.len(), 1);
    }

    #[test]
    fn test_set_retention_policy_replace() {
        let mut inv = ControlInventory::new();
        inv.set_retention_policy(DataRetentionPolicy {
            resource_type: "audit_log".to_string(),
            retention_days: 90,
            pii_redaction: false,
            archive_after_days: None,
        });
        inv.set_retention_policy(DataRetentionPolicy {
            resource_type: "audit_log".to_string(),
            retention_days: 365,
            pii_redaction: true,
            archive_after_days: Some(60),
        });
        assert_eq!(inv.retention_policies.len(), 1);
        assert_eq!(inv.retention_policies[0].retention_days, 365);
    }

    #[test]
    fn test_assess_control() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        let result = inv.assess_control(
            "CC1.1",
            ControlStatus::Implemented,
            vec!["RBAC config screenshot".to_string()],
        );
        assert!(result.is_ok());
        let ctrl = inv.controls.iter().find(|c| c.id == "CC1.1").unwrap();
        assert_eq!(ctrl.status, ControlStatus::Implemented);
        assert_eq!(ctrl.evidence.len(), 1);
        assert!(ctrl.last_assessed > 0);
    }

    #[test]
    fn test_assess_control_not_found() {
        let mut inv = ControlInventory::new();
        let result = inv.assess_control("nonexistent", ControlStatus::Implemented, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_report_all_not_implemented() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        let report = inv.generate_report();
        assert_eq!(report.total_controls, 15);
        assert_eq!(report.implemented, 0);
        assert_eq!(report.not_implemented, 15);
        assert!((report.score_percent - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generate_report_mixed() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        inv.assess_control("CC1.1", ControlStatus::Implemented, vec![]).unwrap();
        inv.assess_control("CC1.2", ControlStatus::Implemented, vec![]).unwrap();
        inv.assess_control("CC1.3", ControlStatus::PartiallyImplemented, vec![]).unwrap();
        let report = inv.generate_report();
        assert_eq!(report.implemented, 2);
        assert_eq!(report.partial, 1);
        // Score: (2 + 0.5) / 15 * 100 = 16.67%
        assert!((report.score_percent - 16.666666666666668).abs() < 0.01);
    }

    #[test]
    fn test_compliance_score() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        assert!((inv.compliance_score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_export_report_markdown() {
        let mut inv = ControlInventory::new();
        inv.add_default_soc2_controls();
        let report = inv.generate_report();
        let md = ControlInventory::export_report_markdown(&report);
        assert!(md.contains("# SOC 2 Compliance Report"));
        assert!(md.contains("Total Controls"));
        assert!(md.contains("CC1.1"));
        assert!(md.contains("Security"));
    }

    #[test]
    fn test_get_audit_log_range() {
        let mut inv = ControlInventory::new();
        for i in 0..10u64 {
            inv.log_action(AuditLogEntry {
                id: format!("log-{}", i),
                timestamp: i * 100,
                actor: "user".to_string(),
                action: AuditAction::FileModified,
                resource: "file.rs".to_string(),
                details: String::new(),
                ip_address: None,
            });
        }
        let entries = inv.get_audit_log(200, 500);
        assert_eq!(entries.len(), 4); // timestamps 200, 300, 400, 500
    }

    #[test]
    fn test_get_audit_log_empty_range() {
        let mut inv = ControlInventory::new();
        inv.log_action(AuditLogEntry {
            id: "log-1".to_string(),
            timestamp: 1000,
            actor: "user".to_string(),
            action: AuditAction::UserLogin,
            resource: "system".to_string(),
            details: String::new(),
            ip_address: None,
        });
        let entries = inv.get_audit_log(2000, 3000);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_redact_pii_email_mask() {
        let text = "Contact alice@example.com for details";
        let fields = vec![PiiField {
            field_name: "email".to_string(),
            field_type: PiiType::Email,
            redaction_method: RedactionMethod::Mask,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(!result.contains("alice@example.com"));
        assert!(result.contains("Contact"));
    }

    #[test]
    fn test_redact_pii_email_remove() {
        let text = "Email: test@domain.org";
        let fields = vec![PiiField {
            field_name: "email".to_string(),
            field_type: PiiType::Email,
            redaction_method: RedactionMethod::Remove,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("test@domain.org"));
    }

    #[test]
    fn test_redact_pii_api_key() {
        let text = "Key: sk_live_abcdefghijklmnopqrstuvwxyz123456";
        let fields = vec![PiiField {
            field_name: "api_key".to_string(),
            field_type: PiiType::ApiKey,
            redaction_method: RedactionMethod::Remove,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_pii_ip_address() {
        let text = "Request from 192.168.1.100 was blocked";
        let fields = vec![PiiField {
            field_name: "ip".to_string(),
            field_type: PiiType::IpAddress,
            redaction_method: RedactionMethod::Remove,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(!result.contains("192.168.1.100"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_pii_name() {
        let text = "User John Smith accessed the system";
        let fields = vec![PiiField {
            field_name: "John Smith".to_string(),
            field_type: PiiType::Name,
            redaction_method: RedactionMethod::Remove,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(!result.contains("John Smith"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_pii_custom() {
        let text = "SSN: 123-45-6789";
        let fields = vec![PiiField {
            field_name: "123-45-6789".to_string(),
            field_type: PiiType::Custom("SSN".to_string()),
            redaction_method: RedactionMethod::Hash,
        }];
        let result = ControlInventory::redact_pii(text, &fields);
        assert!(!result.contains("123-45-6789"));
        assert!(result.contains("[HASH:"));
    }

    #[test]
    fn test_purge_expired_logs() {
        let mut inv = ControlInventory::new();
        inv.set_retention_policy(DataRetentionPolicy {
            resource_type: "audit_log".to_string(),
            retention_days: 1,
            pii_redaction: false,
            archive_after_days: None,
        });
        // Add a log from 2 days ago (172800 seconds)
        inv.log_action(AuditLogEntry {
            id: "old".to_string(),
            timestamp: 0,
            actor: "user".to_string(),
            action: AuditAction::UserLogin,
            resource: "sys".to_string(),
            details: String::new(),
            ip_address: None,
        });
        // Add a recent log
        inv.log_action(AuditLogEntry {
            id: "new".to_string(),
            timestamp: 200000,
            actor: "user".to_string(),
            action: AuditAction::UserLogout,
            resource: "sys".to_string(),
            details: String::new(),
            ip_address: None,
        });
        let purged = inv.purge_expired_logs(200000);
        assert_eq!(purged, 1);
        assert_eq!(inv.audit_log.len(), 1);
        assert_eq!(inv.audit_log[0].id, "new");
    }

    #[test]
    fn test_purge_no_policy_uses_default() {
        let mut inv = ControlInventory::new();
        inv.log_action(AuditLogEntry {
            id: "log".to_string(),
            timestamp: 1000,
            actor: "user".to_string(),
            action: AuditAction::UserLogin,
            resource: "sys".to_string(),
            details: String::new(),
            ip_address: None,
        });
        // With default 365-day retention, recent log should not be purged
        let purged = inv.purge_expired_logs(100000);
        assert_eq!(purged, 0);
    }

    #[test]
    fn test_is_ipv4() {
        assert!(ControlInventory::is_ipv4("192.168.1.1"));
        assert!(ControlInventory::is_ipv4("10.0.0.1"));
        assert!(ControlInventory::is_ipv4("0.0.0.0"));
        assert!(!ControlInventory::is_ipv4("256.1.1.1"));
        assert!(!ControlInventory::is_ipv4("not.an.ip"));
        assert!(!ControlInventory::is_ipv4("1.2.3"));
        assert!(!ControlInventory::is_ipv4(""));
    }

    #[test]
    fn test_trust_criteria_labels() {
        assert_eq!(TrustServiceCriteria::Security.label(), "Security");
        assert_eq!(TrustServiceCriteria::Privacy.label(), "Privacy");
        assert_eq!(
            TrustServiceCriteria::ProcessingIntegrity.label(),
            "Processing Integrity"
        );
    }

    #[test]
    fn test_control_status_labels() {
        assert_eq!(ControlStatus::Implemented.label(), "Implemented");
        assert_eq!(
            ControlStatus::PartiallyImplemented.label(),
            "Partially Implemented"
        );
        assert_eq!(ControlStatus::NotApplicable.label(), "Not Applicable");
    }

    #[test]
    fn test_audit_action_labels() {
        assert_eq!(AuditAction::CodeGenerated.label(), "code_generated");
        assert_eq!(AuditAction::ApiKeyRotated.label(), "api_key_rotated");
        assert_eq!(AuditAction::BudgetModified.label(), "budget_modified");
    }

    #[test]
    fn test_report_with_not_applicable() {
        let mut inv = ControlInventory::new();
        inv.add_control(ComplianceControl {
            id: "C1".to_string(),
            criteria: TrustServiceCriteria::Security,
            title: "Test".to_string(),
            description: "Test".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![],
            last_assessed: 0,
        });
        inv.add_control(ComplianceControl {
            id: "C2".to_string(),
            criteria: TrustServiceCriteria::Privacy,
            title: "NA Test".to_string(),
            description: "Not applicable".to_string(),
            status: ControlStatus::NotApplicable,
            evidence: vec![],
            last_assessed: 0,
        });
        let report = inv.generate_report();
        // 1 implemented out of 1 applicable = 100%
        assert!((report.score_percent - 100.0).abs() < f64::EPSILON);
    }
}
