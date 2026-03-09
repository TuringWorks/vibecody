#![allow(dead_code, clippy::upper_case_acronyms)]
//! Compliance report generation for SOC2/FedRAMP preparation.

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceFramework {
    SOC2,
    FedRAMP,
    HIPAA,
    GDPR,
    ISO27001,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ControlStatus,
    pub evidence: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlStatus {
    Implemented,
    PartiallyImplemented,
    NotImplemented,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub framework: ComplianceFramework,
    pub generated_at: u64,
    pub controls: Vec<ComplianceControl>,
    pub summary: ComplianceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_controls: usize,
    pub implemented: usize,
    pub partial: usize,
    pub not_implemented: usize,
    pub not_applicable: usize,
    pub compliance_percentage: f64,
}

/// Generate a SOC2 compliance report based on VibeCody's security features.
pub fn generate_soc2_report() -> ComplianceReport {
    let controls = vec![
        ComplianceControl {
            id: "CC1.1".to_string(),
            name: "Security Governance".to_string(),
            description: "Organization demonstrates commitment to integrity and ethical values"
                .to_string(),
            status: ControlStatus::Implemented,
            evidence: vec!["MIT License".to_string(), "Open source codebase".to_string()],
            notes: "Fully open source with transparent development".to_string(),
        },
        ComplianceControl {
            id: "CC6.1".to_string(),
            name: "Logical Access Security".to_string(),
            description: "Logical access security over information assets".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "Bearer token authentication (serve.rs)".to_string(),
                "CORS localhost restriction".to_string(),
                "Rate limiting (60 req/60s)".to_string(),
            ],
            notes: "API endpoints protected with bearer tokens and rate limiting".to_string(),
        },
        ComplianceControl {
            id: "CC6.6".to_string(),
            name: "Encryption in Transit".to_string(),
            description: "Data transmitted between entities is protected".to_string(),
            status: ControlStatus::PartiallyImplemented,
            evidence: vec![
                "HTTPS supported".to_string(),
                "TLS cert checking (check_tls_cert)".to_string(),
            ],
            notes: "HTTPS available but HTTP used for local development".to_string(),
        },
        ComplianceControl {
            id: "CC6.7".to_string(),
            name: "Encryption at Rest".to_string(),
            description: "Data at rest is protected".to_string(),
            status: ControlStatus::PartiallyImplemented,
            evidence: vec!["Config file permissions 0o600 (config.rs)".to_string()],
            notes: "File permissions enforced; full encryption depends on OS".to_string(),
        },
        ComplianceControl {
            id: "CC7.2".to_string(),
            name: "Security Monitoring".to_string(),
            description: "System monitoring and anomaly detection".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "OpenTelemetry tracing (otel.rs)".to_string(),
                "Session audit trail (session_store.rs)".to_string(),
                "Secret redaction in logs (trace.rs)".to_string(),
            ],
            notes: "Full observability pipeline with OTLP export and secret redaction".to_string(),
        },
        ComplianceControl {
            id: "CC8.1".to_string(),
            name: "Change Management".to_string(),
            description: "Changes to infrastructure and software are authorized".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "Approval policy system (policy.rs)".to_string(),
                "Hooks pre/post execution (hooks.rs)".to_string(),
                "Git checkpoint system".to_string(),
            ],
            notes: "Multi-level approval policies with hook-based authorization".to_string(),
        },
        ComplianceControl {
            id: "CC9.1".to_string(),
            name: "Risk Mitigation".to_string(),
            description: "Risk mitigation activities are implemented".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "Command blocklist (tool_executor.rs)".to_string(),
                "Path traversal prevention (safe_resolve_path)".to_string(),
                "Sandbox mode (--sandbox flag)".to_string(),
                "Red team scanning (redteam.rs)".to_string(),
            ],
            notes: "Multiple layers of security controls".to_string(),
        },
    ];

    build_report(ComplianceFramework::SOC2, controls)
}

/// Generate a FedRAMP compliance report based on VibeCody's security features.
pub fn generate_fedramp_report() -> ComplianceReport {
    let controls = vec![
        ComplianceControl {
            id: "AC-2".to_string(),
            name: "Account Management".to_string(),
            description: "Manage information system accounts".to_string(),
            status: ControlStatus::PartiallyImplemented,
            evidence: vec![
                "Bearer token auth (serve.rs)".to_string(),
                "Cryptographic session IDs".to_string(),
            ],
            notes: "Token-based access; no multi-user account management yet".to_string(),
        },
        ComplianceControl {
            id: "AU-2".to_string(),
            name: "Audit Events".to_string(),
            description: "Determine auditable events".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "JSONL trace logging".to_string(),
                "SQLite session store".to_string(),
                "OpenTelemetry spans".to_string(),
            ],
            notes: "Comprehensive audit trail for all agent actions".to_string(),
        },
        ComplianceControl {
            id: "SC-13".to_string(),
            name: "Cryptographic Protection".to_string(),
            description: "Use cryptographic mechanisms to protect information".to_string(),
            status: ControlStatus::PartiallyImplemented,
            evidence: vec![
                "SHA-256 installer verification".to_string(),
                "TLS for API calls".to_string(),
            ],
            notes: "Cryptographic verification for installs; TLS for transit".to_string(),
        },
        ComplianceControl {
            id: "SI-10".to_string(),
            name: "Information Input Validation".to_string(),
            description: "Check validity of information inputs".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "Path traversal prevention".to_string(),
                "Command blocklist (8 regex patterns)".to_string(),
                "XSS escape in session viewer".to_string(),
            ],
            notes: "Input validation at multiple layers".to_string(),
        },
        ComplianceControl {
            id: "RA-5".to_string(),
            name: "Vulnerability Scanning".to_string(),
            description: "Scan for vulnerabilities in the system".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec![
                "Red team module (redteam.rs)".to_string(),
                "BugBot OWASP/CWE scanner (bugbot.rs)".to_string(),
                "cargo audit CI job".to_string(),
            ],
            notes: "Automated vulnerability scanning with 15 CWE patterns".to_string(),
        },
    ];

    build_report(ComplianceFramework::FedRAMP, controls)
}

/// Generate a report for a given framework string (used by REPL and Tauri).
pub fn generate_report_for(framework: &str) -> Result<ComplianceReport> {
    match framework.to_lowercase().as_str() {
        "soc2" | "soc 2" => Ok(generate_soc2_report()),
        "fedramp" | "fed_ramp" => Ok(generate_fedramp_report()),
        _ => anyhow::bail!(
            "Unsupported framework: {}. Supported: soc2, fedramp",
            framework
        ),
    }
}

fn build_report(
    framework: ComplianceFramework,
    controls: Vec<ComplianceControl>,
) -> ComplianceReport {
    let total = controls.len();
    let implemented = controls
        .iter()
        .filter(|c| matches!(c.status, ControlStatus::Implemented))
        .count();
    let partial = controls
        .iter()
        .filter(|c| matches!(c.status, ControlStatus::PartiallyImplemented))
        .count();
    let not_impl = controls
        .iter()
        .filter(|c| matches!(c.status, ControlStatus::NotImplemented))
        .count();
    let na = controls
        .iter()
        .filter(|c| matches!(c.status, ControlStatus::NotApplicable))
        .count();
    let applicable = total - na;
    let pct = if applicable > 0 {
        ((implemented as f64 + partial as f64 * 0.5) / applicable as f64) * 100.0
    } else {
        100.0
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    ComplianceReport {
        framework,
        generated_at: now,
        controls,
        summary: ComplianceSummary {
            total_controls: total,
            implemented,
            partial,
            not_implemented: not_impl,
            not_applicable: na,
            compliance_percentage: pct,
        },
    }
}

/// Export report as markdown.
pub fn report_to_markdown(report: &ComplianceReport) -> String {
    let mut md = format!("# {:?} Compliance Report\n\n", report.framework);
    md.push_str(&format!(
        "**Compliance: {:.1}%** ({} implemented, {} partial, {} gaps)\n\n",
        report.summary.compliance_percentage,
        report.summary.implemented,
        report.summary.partial,
        report.summary.not_implemented,
    ));
    md.push_str("| ID | Control | Status | Evidence |\n");
    md.push_str("|---|---|---|---|\n");
    for c in &report.controls {
        let status_str = match c.status {
            ControlStatus::Implemented => "Implemented",
            ControlStatus::PartiallyImplemented => "Partial",
            ControlStatus::NotImplemented => "Gap",
            ControlStatus::NotApplicable => "N/A",
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            c.id,
            c.name,
            status_str,
            c.evidence.join(", ")
        ));
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soc2_report() {
        let report = generate_soc2_report();
        assert_eq!(report.framework, ComplianceFramework::SOC2);
        assert!(!report.controls.is_empty());
        assert!(report.summary.total_controls > 0);
        assert!(report.generated_at > 0);
        assert_eq!(
            report.summary.total_controls,
            report.summary.implemented
                + report.summary.partial
                + report.summary.not_implemented
                + report.summary.not_applicable
        );
    }

    #[test]
    fn test_compliance_percentage() {
        // SOC2 report has 5 Implemented + 2 PartiallyImplemented = 7 controls, 0 N/A
        // pct = (5 + 2*0.5) / 7 * 100 = 6/7 * 100 = 85.71...
        let report = generate_soc2_report();
        let expected = (5.0 + 2.0 * 0.5) / 7.0 * 100.0;
        assert!(
            (report.summary.compliance_percentage - expected).abs() < 0.01,
            "Expected {:.2}, got {:.2}",
            expected,
            report.summary.compliance_percentage
        );
    }

    #[test]
    fn test_report_to_markdown() {
        let report = generate_soc2_report();
        let md = report_to_markdown(&report);
        assert!(md.contains("# SOC2 Compliance Report"));
        assert!(md.contains("| ID | Control | Status | Evidence |"));
        assert!(md.contains("CC1.1"));
        assert!(md.contains("Security Governance"));
        assert!(md.contains("Implemented"));
        assert!(md.contains("Partial"));
    }

    #[test]
    fn test_control_serde() {
        let control = ComplianceControl {
            id: "TEST-1".to_string(),
            name: "Test Control".to_string(),
            description: "A test".to_string(),
            status: ControlStatus::Implemented,
            evidence: vec!["evidence1".to_string()],
            notes: "notes".to_string(),
        };
        let json = serde_json::to_string(&control).expect("serialize");
        let deserialized: ComplianceControl =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, "TEST-1");
        assert_eq!(deserialized.status, ControlStatus::Implemented);
        assert_eq!(deserialized.evidence.len(), 1);
    }

    #[test]
    fn test_fedramp_report() {
        let report = generate_fedramp_report();
        assert_eq!(report.framework, ComplianceFramework::FedRAMP);
        assert!(!report.controls.is_empty());
        assert!(report.summary.total_controls > 0);
    }

    #[test]
    fn test_generate_report_for() {
        let soc2 = generate_report_for("soc2").unwrap();
        assert_eq!(soc2.framework, ComplianceFramework::SOC2);

        let fedramp = generate_report_for("fedramp").unwrap();
        assert_eq!(fedramp.framework, ComplianceFramework::FedRAMP);

        let err = generate_report_for("unknown");
        assert!(err.is_err());
    }

    #[test]
    fn test_generate_report_for_case_variants() {
        assert!(generate_report_for("SOC2").is_ok());
        assert!(generate_report_for("Soc2").is_ok());
        assert!(generate_report_for("soc 2").is_ok());
        assert!(generate_report_for("FedRAMP").is_ok());
        assert!(generate_report_for("FEDRAMP").is_ok());
        assert!(generate_report_for("fed_ramp").is_ok());
    }

    #[test]
    fn test_generate_report_for_unsupported() {
        let err = generate_report_for("hipaa");
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("Unsupported framework"));
    }

    #[test]
    fn test_soc2_control_ids() {
        let report = generate_soc2_report();
        let ids: Vec<&str> = report.controls.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"CC1.1"));
        assert!(ids.contains(&"CC6.1"));
        assert!(ids.contains(&"CC6.6"));
        assert!(ids.contains(&"CC9.1"));
    }

    #[test]
    fn test_fedramp_control_ids() {
        let report = generate_fedramp_report();
        let ids: Vec<&str> = report.controls.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"AC-2"));
        assert!(ids.contains(&"AU-2"));
        assert!(ids.contains(&"SC-13"));
        assert!(ids.contains(&"SI-10"));
        assert!(ids.contains(&"RA-5"));
    }

    #[test]
    fn test_fedramp_summary_counts() {
        let report = generate_fedramp_report();
        let s = &report.summary;
        assert_eq!(
            s.total_controls,
            s.implemented + s.partial + s.not_implemented + s.not_applicable
        );
    }

    #[test]
    fn test_fedramp_compliance_percentage() {
        let report = generate_fedramp_report();
        assert!(report.summary.compliance_percentage > 0.0);
        assert!(report.summary.compliance_percentage <= 100.0);
    }

    #[test]
    fn test_report_to_markdown_fedramp() {
        let report = generate_fedramp_report();
        let md = report_to_markdown(&report);
        assert!(md.contains("FedRAMP Compliance Report"));
        assert!(md.contains("AC-2"));
        assert!(md.contains("Account Management"));
    }

    #[test]
    fn test_control_status_serde_all_variants() {
        for status in [
            ControlStatus::Implemented,
            ControlStatus::PartiallyImplemented,
            ControlStatus::NotImplemented,
            ControlStatus::NotApplicable,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: ControlStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_framework_serde_all_variants() {
        for fw in [
            ComplianceFramework::SOC2,
            ComplianceFramework::FedRAMP,
            ComplianceFramework::HIPAA,
            ComplianceFramework::GDPR,
            ComplianceFramework::ISO27001,
        ] {
            let json = serde_json::to_string(&fw).unwrap();
            let parsed: ComplianceFramework = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, fw);
        }
    }

    #[test]
    fn test_report_serde_roundtrip() {
        let report = generate_soc2_report();
        let json = serde_json::to_string(&report).unwrap();
        let parsed: ComplianceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.framework, ComplianceFramework::SOC2);
        assert_eq!(parsed.controls.len(), report.controls.len());
        assert!((parsed.summary.compliance_percentage - report.summary.compliance_percentage).abs() < 0.001);
    }

    #[test]
    fn test_markdown_contains_table_headers() {
        let report = generate_soc2_report();
        let md = report_to_markdown(&report);
        assert!(md.contains("| ID | Control | Status | Evidence |"));
        assert!(md.contains("|---|---|---|---|"));
    }

    #[test]
    fn test_control_with_empty_evidence() {
        let control = ComplianceControl {
            id: "T-1".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            status: ControlStatus::NotApplicable,
            evidence: vec![],
            notes: String::new(),
        };
        let json = serde_json::to_string(&control).unwrap();
        let parsed: ComplianceControl = serde_json::from_str(&json).unwrap();
        assert!(parsed.evidence.is_empty());
        assert_eq!(parsed.status, ControlStatus::NotApplicable);
    }
}
