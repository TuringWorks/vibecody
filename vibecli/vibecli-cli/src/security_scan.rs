#![allow(dead_code)]

use std::time::SystemTime;

/// Type of security scan to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanType {
    SecretDetection,
    DependencyVulnerability,
    StaticAnalysis,
    LicenseCompliance,
    ConfigAudit,
}

/// Severity level of a security finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScanSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl ScanSeverity {
    pub fn score(&self) -> u32 {
        match self {
            ScanSeverity::Critical => 10,
            ScanSeverity::High => 7,
            ScanSeverity::Medium => 4,
            ScanSeverity::Low => 2,
            ScanSeverity::Info => 0,
        }
    }

    pub fn is_at_least(&self, other: &ScanSeverity) -> bool {
        self.score() >= other.score()
    }
}

/// Status of a scan execution.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Skipped,
}

/// A single finding from a security scan.
#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub id: String,
    pub scan_type: ScanType,
    pub severity: ScanSeverity,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line: Option<usize>,
    pub remediation: Option<String>,
    pub cve_id: Option<String>,
    pub cwe_id: Option<String>,
    pub false_positive: bool,
    pub suppressed: bool,
}

impl SecurityFinding {
    pub fn new(scan_type: ScanType, severity: ScanSeverity, title: &str, desc: &str) -> Self {
        let id = format!(
            "finding-{}-{}",
            title.replace(' ', "-").to_lowercase(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_millis()
                % 100_000
        );
        Self {
            id,
            scan_type,
            severity,
            title: title.to_string(),
            description: desc.to_string(),
            file_path: None,
            line: None,
            remediation: None,
            cve_id: None,
            cwe_id: None,
            false_positive: false,
            suppressed: false,
        }
    }

    pub fn with_file(mut self, path: &str, line: usize) -> Self {
        self.file_path = Some(path.to_string());
        self.line = Some(line);
        self
    }

    pub fn with_cve(mut self, cve: &str) -> Self {
        self.cve_id = Some(cve.to_string());
        self
    }

    pub fn suppress(&mut self) {
        self.suppressed = true;
    }

    pub fn mark_false_positive(&mut self) {
        self.false_positive = true;
        self.suppressed = true;
    }

    pub fn is_blocking(&self, threshold: &ScanSeverity) -> bool {
        !self.suppressed && !self.false_positive && self.severity.is_at_least(threshold)
    }
}

/// Pattern for detecting secrets in source code.
#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub name: String,
    pub pattern: String,
    pub severity: ScanSeverity,
    pub description: String,
}

impl SecretPattern {
    pub fn default_patterns() -> Vec<SecretPattern> {
        vec![
            SecretPattern {
                name: "AWS Access Key".to_string(),
                pattern: "AKIA[0-9A-Z]{16}".to_string(),
                severity: ScanSeverity::Critical,
                description: "AWS access key ID detected".to_string(),
            },
            SecretPattern {
                name: "AWS Secret Key".to_string(),
                pattern: "aws_secret_access_key".to_string(),
                severity: ScanSeverity::Critical,
                description: "AWS secret access key reference detected".to_string(),
            },
            SecretPattern {
                name: "GitHub Token".to_string(),
                pattern: "ghp_[0-9a-zA-Z]{36}".to_string(),
                severity: ScanSeverity::Critical,
                description: "GitHub personal access token detected".to_string(),
            },
            SecretPattern {
                name: "GitHub OAuth".to_string(),
                pattern: "gho_[0-9a-zA-Z]{36}".to_string(),
                severity: ScanSeverity::High,
                description: "GitHub OAuth token detected".to_string(),
            },
            SecretPattern {
                name: "Private Key".to_string(),
                pattern: "-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----".to_string(),
                severity: ScanSeverity::Critical,
                description: "Private key detected in source code".to_string(),
            },
            SecretPattern {
                name: "Slack Webhook".to_string(),
                pattern: "hooks.slack.com/services/T[A-Z0-9]{8}/B[A-Z0-9]{8}".to_string(),
                severity: ScanSeverity::High,
                description: "Slack webhook URL detected".to_string(),
            },
            SecretPattern {
                name: "Generic API Key".to_string(),
                pattern: "api[_-]?key['\"]?\\s*[:=]\\s*['\"][a-zA-Z0-9]{20,}".to_string(),
                severity: ScanSeverity::High,
                description: "Generic API key assignment detected".to_string(),
            },
            SecretPattern {
                name: "Password Assignment".to_string(),
                pattern: "password['\"]?\\s*[:=]\\s*['\"][^'\"]{8,}".to_string(),
                severity: ScanSeverity::High,
                description: "Hardcoded password detected".to_string(),
            },
            SecretPattern {
                name: "JWT Token".to_string(),
                pattern: "eyJ[A-Za-z0-9_-]{10,}\\.eyJ[A-Za-z0-9_-]{10,}\\.[A-Za-z0-9_-]{10,}".to_string(),
                severity: ScanSeverity::High,
                description: "JWT token detected in source code".to_string(),
            },
            SecretPattern {
                name: "Database Connection String".to_string(),
                pattern: "(mysql|postgres|mongodb)(\\+srv)?://[^\\s'\"]{10,}".to_string(),
                severity: ScanSeverity::Critical,
                description: "Database connection string with credentials detected".to_string(),
            },
            SecretPattern {
                name: "Stripe Key".to_string(),
                pattern: "sk_live_[0-9a-zA-Z]{24,}".to_string(),
                severity: ScanSeverity::Critical,
                description: "Stripe live secret key detected".to_string(),
            },
            SecretPattern {
                name: "SendGrid Key".to_string(),
                pattern: "SG\\.[a-zA-Z0-9_-]{22}\\.[a-zA-Z0-9_-]{43}".to_string(),
                severity: ScanSeverity::High,
                description: "SendGrid API key detected".to_string(),
            },
        ]
    }

    /// Check if a line matches this secret pattern using simple substring/prefix matching.
    /// For production use this would use a regex engine; here we do deterministic substring checks.
    pub fn matches(&self, line: &str) -> bool {
        // Simple heuristic matching for common patterns without regex crate
        match self.name.as_str() {
            "AWS Access Key" => {
                line.contains("AKIA")
                    && line.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 20
            }
            "AWS Secret Key" => line.to_lowercase().contains("aws_secret_access_key"),
            "GitHub Token" => line.contains("ghp_") && {
                if let Some(idx) = line.find("ghp_") {
                    line[idx + 4..].len() >= 36
                } else {
                    false
                }
            },
            "GitHub OAuth" => line.contains("gho_") && {
                if let Some(idx) = line.find("gho_") {
                    line[idx + 4..].len() >= 36
                } else {
                    false
                }
            },
            "Private Key" => line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----"),
            "Slack Webhook" => line.contains("hooks.slack.com/services/"),
            "Generic API Key" => {
                let lower = line.to_lowercase();
                (lower.contains("api_key") || lower.contains("api-key") || lower.contains("apikey"))
                    && (lower.contains('=') || lower.contains(':'))
            }
            "Password Assignment" => {
                let lower = line.to_lowercase();
                lower.contains("password")
                    && (lower.contains('=') || lower.contains(':'))
                    && !lower.contains("password_hash")
                    && !lower.contains("password_reset")
            }
            "JWT Token" => {
                line.contains("eyJ") && line.matches('.').count() >= 2
            }
            "Database Connection String" => {
                let lower = line.to_lowercase();
                lower.contains("mysql://")
                    || lower.contains("postgres://")
                    || lower.contains("mongodb://")
                    || lower.contains("mongodb+srv://")
            }
            "Stripe Key" => line.contains("sk_live_"),
            "SendGrid Key" => line.starts_with("SG.") || line.contains("SG."),
            _ => line.contains(&self.pattern),
        }
    }
}

/// Known dependency vulnerability entry.
#[derive(Debug, Clone)]
pub struct DependencyVulnerability {
    pub package: String,
    pub version: String,
    pub vulnerability: String,
    pub severity: ScanSeverity,
    pub fixed_version: Option<String>,
    pub description: String,
}

/// Result of a single scan run.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub id: String,
    pub scan_type: ScanType,
    pub status: ScanStatus,
    pub findings: Vec<SecurityFinding>,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub files_scanned: usize,
    pub lines_scanned: usize,
}

impl ScanResult {
    pub fn new(scan_type: ScanType) -> Self {
        let id = format!(
            "scan-{:?}-{}",
            scan_type,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_millis()
                % 100_000
        );
        Self {
            id,
            scan_type,
            status: ScanStatus::Pending,
            findings: Vec::new(),
            started_at: None,
            completed_at: None,
            files_scanned: 0,
            lines_scanned: 0,
        }
    }

    pub fn start(&mut self) {
        self.status = ScanStatus::Running;
        self.started_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.status = ScanStatus::Completed;
        self.completed_at = Some(SystemTime::now());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = ScanStatus::Failed(error.to_string());
        self.completed_at = Some(SystemTime::now());
    }

    pub fn add_finding(&mut self, finding: SecurityFinding) {
        self.findings.push(finding);
    }

    pub fn findings_by_severity(&self, sev: &ScanSeverity) -> Vec<&SecurityFinding> {
        self.findings
            .iter()
            .filter(|f| &f.severity == sev)
            .collect()
    }

    pub fn has_blocking_findings(&self, threshold: &ScanSeverity) -> bool {
        self.findings.iter().any(|f| f.is_blocking(threshold))
    }
}

/// Configuration for the security scanner.
#[derive(Debug, Clone)]
pub struct SecurityScanConfig {
    pub enabled_scans: Vec<ScanType>,
    pub secret_patterns: Vec<SecretPattern>,
    pub ignore_paths: Vec<String>,
    pub severity_threshold: ScanSeverity,
    pub fail_on_severity: ScanSeverity,
    pub suppress_known: bool,
    pub max_findings: usize,
}

impl SecurityScanConfig {
    pub fn default_config() -> Self {
        Self {
            enabled_scans: vec![
                ScanType::SecretDetection,
                ScanType::DependencyVulnerability,
                ScanType::StaticAnalysis,
            ],
            secret_patterns: SecretPattern::default_patterns(),
            ignore_paths: vec![
                "node_modules/".to_string(),
                "target/".to_string(),
                ".git/".to_string(),
                "vendor/".to_string(),
                "dist/".to_string(),
            ],
            severity_threshold: ScanSeverity::Low,
            fail_on_severity: ScanSeverity::High,
            suppress_known: true,
            max_findings: 1000,
        }
    }

    pub fn strict() -> Self {
        Self {
            enabled_scans: vec![
                ScanType::SecretDetection,
                ScanType::DependencyVulnerability,
                ScanType::StaticAnalysis,
                ScanType::LicenseCompliance,
                ScanType::ConfigAudit,
            ],
            secret_patterns: SecretPattern::default_patterns(),
            ignore_paths: vec![".git/".to_string()],
            severity_threshold: ScanSeverity::Info,
            fail_on_severity: ScanSeverity::Medium,
            suppress_known: false,
            max_findings: 5000,
        }
    }

    pub fn permissive() -> Self {
        Self {
            enabled_scans: vec![ScanType::SecretDetection],
            secret_patterns: SecretPattern::default_patterns(),
            ignore_paths: vec![
                "node_modules/".to_string(),
                "target/".to_string(),
                ".git/".to_string(),
                "vendor/".to_string(),
                "dist/".to_string(),
                "test/".to_string(),
                "tests/".to_string(),
                "spec/".to_string(),
            ],
            severity_threshold: ScanSeverity::High,
            fail_on_severity: ScanSeverity::Critical,
            suppress_known: true,
            max_findings: 500,
        }
    }
}

/// Aggregate report from all scan results.
#[derive(Debug, Clone)]
pub struct ScanReport {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub blocked: bool,
    pub summary: String,
}

/// Main security scanner that orchestrates scans and manages results.
#[derive(Debug)]
pub struct SecurityScanner {
    pub config: SecurityScanConfig,
    pub results: Vec<ScanResult>,
    pub suppressed_ids: Vec<String>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        Self {
            config: SecurityScanConfig::default_config(),
            results: Vec::new(),
            suppressed_ids: Vec::new(),
        }
    }

    /// Scan content for secrets, returning a ScanResult with findings.
    pub fn scan_content(&mut self, content: &str, file_path: &str) -> ScanResult {
        let mut result = ScanResult::new(ScanType::SecretDetection);
        result.start();
        result.files_scanned = 1;

        // Check if the file path should be ignored
        for ignore in &self.config.ignore_paths {
            if file_path.contains(ignore.trim_end_matches('/')) {
                result.status = ScanStatus::Skipped;
                return result;
            }
        }

        let lines: Vec<&str> = content.lines().collect();
        result.lines_scanned = lines.len();

        for (line_num, line) in lines.iter().enumerate() {
            if result.findings.len() >= self.config.max_findings {
                break;
            }
            for pattern in &self.config.secret_patterns {
                if pattern.matches(line) {
                    if !pattern.severity.is_at_least(&self.config.severity_threshold) {
                        continue;
                    }
                    let mut finding = SecurityFinding::new(
                        ScanType::SecretDetection,
                        pattern.severity.clone(),
                        &pattern.name,
                        &pattern.description,
                    )
                    .with_file(file_path, line_num + 1);
                    finding.remediation = Some(format!(
                        "Remove the {} from source code and use environment variables or a secrets manager instead",
                        pattern.name.to_lowercase()
                    ));
                    if self.suppressed_ids.contains(&finding.id) {
                        finding.suppress();
                    }
                    result.add_finding(finding);
                }
            }
        }

        result.complete();
        self.results.push(result.clone());
        result
    }

    /// Scan dependency file content for known vulnerable patterns.
    pub fn scan_dependencies(&mut self, deps_content: &str, format: &str) -> ScanResult {
        let mut result = ScanResult::new(ScanType::DependencyVulnerability);
        result.start();
        result.files_scanned = 1;
        result.lines_scanned = deps_content.lines().count();

        // Known vulnerable package patterns (simplified without serde)
        let known_vulns: Vec<DependencyVulnerability> = vec![
            DependencyVulnerability {
                package: "lodash".to_string(),
                version: "4.17.20".to_string(),
                vulnerability: "CVE-2021-23337".to_string(),
                severity: ScanSeverity::High,
                fixed_version: Some("4.17.21".to_string()),
                description: "Prototype pollution in lodash".to_string(),
            },
            DependencyVulnerability {
                package: "minimist".to_string(),
                version: "1.2.5".to_string(),
                vulnerability: "CVE-2021-44906".to_string(),
                severity: ScanSeverity::Critical,
                fixed_version: Some("1.2.6".to_string()),
                description: "Prototype pollution in minimist".to_string(),
            },
            DependencyVulnerability {
                package: "node-forge".to_string(),
                version: "1.2.1".to_string(),
                vulnerability: "CVE-2022-24771".to_string(),
                severity: ScanSeverity::High,
                fixed_version: Some("1.3.0".to_string()),
                description: "Signature verification bypass in node-forge".to_string(),
            },
            DependencyVulnerability {
                package: "flask".to_string(),
                version: "2.0.0".to_string(),
                vulnerability: "CVE-2023-30861".to_string(),
                severity: ScanSeverity::High,
                fixed_version: Some("2.3.2".to_string()),
                description: "Cookie handling vulnerability in Flask".to_string(),
            },
        ];

        let lower_content = deps_content.to_lowercase();

        for vuln in &known_vulns {
            let pkg_lower = vuln.package.to_lowercase();
            let matches = match format {
                "package.json" => {
                    lower_content.contains(&format!("\"{}\"", pkg_lower))
                        && lower_content.contains(&vuln.version)
                }
                "requirements.txt" => {
                    lower_content.contains(&pkg_lower)
                        && lower_content.contains(&vuln.version)
                }
                "Cargo.toml" => {
                    lower_content.contains(&pkg_lower)
                        && lower_content.contains(&vuln.version)
                }
                _ => lower_content.contains(&pkg_lower) && lower_content.contains(&vuln.version),
            };

            if matches {
                let mut finding = SecurityFinding::new(
                    ScanType::DependencyVulnerability,
                    vuln.severity.clone(),
                    &format!("Vulnerable dependency: {} {}", vuln.package, vuln.version),
                    &vuln.description,
                )
                .with_cve(&vuln.vulnerability);
                if let Some(ref fixed) = vuln.fixed_version {
                    finding.remediation = Some(format!(
                        "Upgrade {} to version {} or later",
                        vuln.package, fixed
                    ));
                }
                finding.cwe_id = Some("CWE-1104".to_string());
                result.add_finding(finding);
            }
        }

        result.complete();
        self.results.push(result.clone());
        result
    }

    /// Generate an aggregate report from all scan results.
    pub fn generate_report(&self) -> ScanReport {
        let all: Vec<&SecurityFinding> = self.all_findings();
        let active: Vec<&&SecurityFinding> = all
            .iter()
            .filter(|f| !f.suppressed && !f.false_positive)
            .collect();

        let critical = active.iter().filter(|f| f.severity == ScanSeverity::Critical).count();
        let high = active.iter().filter(|f| f.severity == ScanSeverity::High).count();
        let medium = active.iter().filter(|f| f.severity == ScanSeverity::Medium).count();
        let low = active.iter().filter(|f| f.severity == ScanSeverity::Low).count();
        let info = active.iter().filter(|f| f.severity == ScanSeverity::Info).count();

        let blocked = self.should_block_pr();

        let summary = if active.is_empty() {
            "No security findings detected. All clear!".to_string()
        } else if blocked {
            format!(
                "BLOCKED: {} findings detected ({} critical, {} high, {} medium, {} low, {} info). PR cannot proceed.",
                active.len(), critical, high, medium, low, info
            )
        } else {
            format!(
                "{} findings detected ({} critical, {} high, {} medium, {} low, {} info). PR can proceed.",
                active.len(), critical, high, medium, low, info
            )
        };

        ScanReport {
            total_findings: active.len(),
            critical,
            high,
            medium,
            low,
            info,
            blocked,
            summary,
        }
    }

    pub fn suppress_finding(&mut self, id: &str) {
        self.suppressed_ids.push(id.to_string());
        for result in &mut self.results {
            for finding in &mut result.findings {
                if finding.id == id {
                    finding.suppress();
                }
            }
        }
    }

    pub fn all_findings(&self) -> Vec<&SecurityFinding> {
        self.results
            .iter()
            .flat_map(|r| r.findings.iter())
            .collect()
    }

    pub fn should_block_pr(&self) -> bool {
        self.results
            .iter()
            .any(|r| r.has_blocking_findings(&self.config.fail_on_severity))
    }

    /// Generate a Markdown report of all findings.
    pub fn to_markdown(&self) -> String {
        let report = self.generate_report();
        let mut md = String::new();

        md.push_str("# Security Scan Report\n\n");
        md.push_str(&format!("**Status:** {}\n\n", if report.blocked { "BLOCKED" } else { "PASSED" }));
        md.push_str("## Summary\n\n");
        md.push_str(&format!("| Severity | Count |\n"));
        md.push_str("| --- | --- |\n");
        md.push_str(&format!("| Critical | {} |\n", report.critical));
        md.push_str(&format!("| High | {} |\n", report.high));
        md.push_str(&format!("| Medium | {} |\n", report.medium));
        md.push_str(&format!("| Low | {} |\n", report.low));
        md.push_str(&format!("| Info | {} |\n", report.info));
        md.push_str(&format!("| **Total** | **{}** |\n\n", report.total_findings));

        let all_findings = self.all_findings();
        if !all_findings.is_empty() {
            md.push_str("## Findings\n\n");
            for finding in &all_findings {
                if finding.suppressed || finding.false_positive {
                    continue;
                }
                md.push_str(&format!(
                    "### [{:?}] {}\n\n",
                    finding.severity, finding.title
                ));
                md.push_str(&format!("- **Type:** {:?}\n", finding.scan_type));
                md.push_str(&format!("- **Description:** {}\n", finding.description));
                if let Some(ref path) = finding.file_path {
                    md.push_str(&format!(
                        "- **Location:** {}:{}\n",
                        path,
                        finding.line.unwrap_or(0)
                    ));
                }
                if let Some(ref cve) = finding.cve_id {
                    md.push_str(&format!("- **CVE:** {}\n", cve));
                }
                if let Some(ref cwe) = finding.cwe_id {
                    md.push_str(&format!("- **CWE:** {}\n", cwe));
                }
                if let Some(ref rem) = finding.remediation {
                    md.push_str(&format!("- **Remediation:** {}\n", rem));
                }
                md.push('\n');
            }
        }

        md.push_str(&format!("\n{}\n", report.summary));
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ScanSeverity tests ---

    #[test]
    fn test_severity_score_critical() {
        assert_eq!(ScanSeverity::Critical.score(), 10);
    }

    #[test]
    fn test_severity_score_high() {
        assert_eq!(ScanSeverity::High.score(), 7);
    }

    #[test]
    fn test_severity_score_medium() {
        assert_eq!(ScanSeverity::Medium.score(), 4);
    }

    #[test]
    fn test_severity_score_low() {
        assert_eq!(ScanSeverity::Low.score(), 2);
    }

    #[test]
    fn test_severity_score_info() {
        assert_eq!(ScanSeverity::Info.score(), 0);
    }

    #[test]
    fn test_severity_is_at_least() {
        assert!(ScanSeverity::Critical.is_at_least(&ScanSeverity::High));
        assert!(ScanSeverity::High.is_at_least(&ScanSeverity::High));
        assert!(!ScanSeverity::Medium.is_at_least(&ScanSeverity::High));
        assert!(ScanSeverity::Low.is_at_least(&ScanSeverity::Info));
        assert!(!ScanSeverity::Info.is_at_least(&ScanSeverity::Low));
    }

    // --- SecretPattern tests ---

    #[test]
    fn test_default_patterns_count() {
        let patterns = SecretPattern::default_patterns();
        assert!(patterns.len() >= 10);
    }

    #[test]
    fn test_pattern_matches_aws_key() {
        let patterns = SecretPattern::default_patterns();
        let aws = patterns.iter().find(|p| p.name == "AWS Access Key").unwrap();
        assert!(aws.matches("AKIAIOSFODNN7EXAMPLE1234567890"));
        assert!(!aws.matches("just some normal text"));
    }

    #[test]
    fn test_pattern_matches_aws_secret() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "AWS Secret Key").unwrap();
        assert!(pat.matches("aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"));
        assert!(!pat.matches("just some text"));
    }

    #[test]
    fn test_pattern_matches_github_token() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "GitHub Token").unwrap();
        assert!(pat.matches("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijkl"));
        assert!(!pat.matches("ghp_short"));
    }

    #[test]
    fn test_pattern_matches_private_key() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "Private Key").unwrap();
        assert!(pat.matches("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(pat.matches("-----BEGIN PRIVATE KEY-----"));
        assert!(!pat.matches("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn test_pattern_matches_slack_webhook() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "Slack Webhook").unwrap();
        assert!(pat.matches("https://hooks.slack.com/services/T12345678/B12345678/abcdefgh"));
        assert!(!pat.matches("https://slack.com/api/chat.postMessage"));
    }

    #[test]
    fn test_pattern_matches_generic_api_key() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "Generic API Key").unwrap();
        assert!(pat.matches("api_key = 'sk_1234567890abcdefghij'"));
        assert!(pat.matches("API-KEY: abcdefghijklmnopqrstuvwxyz"));
        assert!(!pat.matches("just a normal line of code"));
    }

    #[test]
    fn test_pattern_matches_password() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "Password Assignment").unwrap();
        assert!(pat.matches("password = 'mysecretpassword123'"));
        assert!(!pat.matches("password_hash = bcrypt(input)"));
    }

    #[test]
    fn test_pattern_matches_jwt() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "JWT Token").unwrap();
        assert!(pat.matches("token = eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature123456"));
        assert!(!pat.matches("just some text without jwt"));
    }

    #[test]
    fn test_pattern_matches_db_connection() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns
            .iter()
            .find(|p| p.name == "Database Connection String")
            .unwrap();
        assert!(pat.matches("postgres://user:pass@localhost:5432/mydb"));
        assert!(pat.matches("mongodb+srv://admin:secret@cluster.mongodb.net/db"));
        assert!(!pat.matches("just a normal string"));
    }

    #[test]
    fn test_pattern_matches_stripe_key() {
        let patterns = SecretPattern::default_patterns();
        let pat = patterns.iter().find(|p| p.name == "Stripe Key").unwrap();
        assert!(pat.matches(&format!("sk_{}_abcdefghijklmnopqrstuvwx", "live")));
        assert!(!pat.matches(&format!("sk_{}_abcdefghijklmnopqrstuvwx", "test")));
    }

    // --- SecurityFinding tests ---

    #[test]
    fn test_finding_new() {
        let f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::High,
            "Test Finding",
            "A test",
        );
        assert!(f.id.starts_with("finding-"));
        assert_eq!(f.severity, ScanSeverity::High);
        assert!(!f.suppressed);
        assert!(!f.false_positive);
    }

    #[test]
    fn test_finding_with_file() {
        let f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::High,
            "Test",
            "desc",
        )
        .with_file("src/main.rs", 42);
        assert_eq!(f.file_path.as_deref(), Some("src/main.rs"));
        assert_eq!(f.line, Some(42));
    }

    #[test]
    fn test_finding_with_cve() {
        let f = SecurityFinding::new(
            ScanType::DependencyVulnerability,
            ScanSeverity::Critical,
            "Test",
            "desc",
        )
        .with_cve("CVE-2024-1234");
        assert_eq!(f.cve_id.as_deref(), Some("CVE-2024-1234"));
    }

    #[test]
    fn test_finding_suppress() {
        let mut f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::High,
            "Test",
            "desc",
        );
        assert!(!f.suppressed);
        f.suppress();
        assert!(f.suppressed);
    }

    #[test]
    fn test_finding_mark_false_positive() {
        let mut f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::High,
            "Test",
            "desc",
        );
        f.mark_false_positive();
        assert!(f.false_positive);
        assert!(f.suppressed);
    }

    #[test]
    fn test_finding_is_blocking() {
        let f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::Critical,
            "Test",
            "desc",
        );
        assert!(f.is_blocking(&ScanSeverity::High));
        assert!(f.is_blocking(&ScanSeverity::Critical));
        assert!(!SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::Medium,
            "Test",
            "desc",
        )
        .is_blocking(&ScanSeverity::High));
    }

    #[test]
    fn test_finding_suppressed_not_blocking() {
        let mut f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::Critical,
            "Test",
            "desc",
        );
        f.suppress();
        assert!(!f.is_blocking(&ScanSeverity::High));
    }

    #[test]
    fn test_finding_false_positive_not_blocking() {
        let mut f = SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::Critical,
            "Test",
            "desc",
        );
        f.mark_false_positive();
        assert!(!f.is_blocking(&ScanSeverity::Info));
    }

    // --- ScanResult tests ---

    #[test]
    fn test_scan_result_new() {
        let r = ScanResult::new(ScanType::SecretDetection);
        assert_eq!(r.scan_type, ScanType::SecretDetection);
        assert_eq!(r.status, ScanStatus::Pending);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn test_scan_result_lifecycle() {
        let mut r = ScanResult::new(ScanType::StaticAnalysis);
        assert_eq!(r.status, ScanStatus::Pending);
        r.start();
        assert_eq!(r.status, ScanStatus::Running);
        assert!(r.started_at.is_some());
        r.complete();
        assert_eq!(r.status, ScanStatus::Completed);
        assert!(r.completed_at.is_some());
    }

    #[test]
    fn test_scan_result_fail() {
        let mut r = ScanResult::new(ScanType::SecretDetection);
        r.start();
        r.fail("timeout");
        assert_eq!(r.status, ScanStatus::Failed("timeout".to_string()));
    }

    #[test]
    fn test_scan_result_add_finding() {
        let mut r = ScanResult::new(ScanType::SecretDetection);
        r.add_finding(SecurityFinding::new(
            ScanType::SecretDetection,
            ScanSeverity::High,
            "test",
            "desc",
        ));
        assert_eq!(r.findings.len(), 1);
    }

    #[test]
    fn test_scan_result_findings_by_severity() {
        let mut r = ScanResult::new(ScanType::SecretDetection);
        r.add_finding(SecurityFinding::new(ScanType::SecretDetection, ScanSeverity::High, "a", "d"));
        r.add_finding(SecurityFinding::new(ScanType::SecretDetection, ScanSeverity::Low, "b", "d"));
        r.add_finding(SecurityFinding::new(ScanType::SecretDetection, ScanSeverity::High, "c", "d"));
        assert_eq!(r.findings_by_severity(&ScanSeverity::High).len(), 2);
        assert_eq!(r.findings_by_severity(&ScanSeverity::Low).len(), 1);
        assert_eq!(r.findings_by_severity(&ScanSeverity::Critical).len(), 0);
    }

    #[test]
    fn test_scan_result_has_blocking() {
        let mut r = ScanResult::new(ScanType::SecretDetection);
        r.add_finding(SecurityFinding::new(ScanType::SecretDetection, ScanSeverity::Low, "a", "d"));
        assert!(!r.has_blocking_findings(&ScanSeverity::High));
        r.add_finding(SecurityFinding::new(ScanType::SecretDetection, ScanSeverity::Critical, "b", "d"));
        assert!(r.has_blocking_findings(&ScanSeverity::High));
    }

    // --- SecurityScanConfig tests ---

    #[test]
    fn test_config_default() {
        let cfg = SecurityScanConfig::default_config();
        assert_eq!(cfg.enabled_scans.len(), 3);
        assert!(cfg.secret_patterns.len() >= 10);
        assert_eq!(cfg.fail_on_severity, ScanSeverity::High);
        assert!(cfg.suppress_known);
        assert_eq!(cfg.max_findings, 1000);
    }

    #[test]
    fn test_config_strict() {
        let cfg = SecurityScanConfig::strict();
        assert_eq!(cfg.enabled_scans.len(), 5);
        assert_eq!(cfg.fail_on_severity, ScanSeverity::Medium);
        assert!(!cfg.suppress_known);
    }

    #[test]
    fn test_config_permissive() {
        let cfg = SecurityScanConfig::permissive();
        assert_eq!(cfg.enabled_scans.len(), 1);
        assert_eq!(cfg.fail_on_severity, ScanSeverity::Critical);
        assert!(cfg.suppress_known);
    }

    // --- SecurityScanner tests ---

    #[test]
    fn test_scanner_new() {
        let s = SecurityScanner::new();
        assert!(s.results.is_empty());
        assert!(s.suppressed_ids.is_empty());
    }

    #[test]
    fn test_scanner_scan_content_clean() {
        let mut s = SecurityScanner::new();
        let result = s.scan_content("fn main() {\n    println!(\"hello\");\n}\n", "src/main.rs");
        assert_eq!(result.status, ScanStatus::Completed);
        assert!(result.findings.is_empty());
        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.lines_scanned, 3);
    }

    #[test]
    fn test_scanner_scan_content_with_aws_key() {
        let mut s = SecurityScanner::new();
        let content = "let key = \"AKIAIOSFODNN7EXAMPLE1234567890\";\n";
        let result = s.scan_content(content, "src/config.rs");
        assert!(!result.findings.is_empty());
        assert_eq!(result.findings[0].severity, ScanSeverity::Critical);
    }

    #[test]
    fn test_scanner_scan_content_with_private_key() {
        let mut s = SecurityScanner::new();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----\n";
        let result = s.scan_content(content, "certs/server.key");
        assert!(!result.findings.is_empty());
    }

    #[test]
    fn test_scanner_scan_content_ignored_path() {
        let mut s = SecurityScanner::new();
        let content = "AKIAIOSFODNN7EXAMPLE1234567890";
        let result = s.scan_content(content, "node_modules/pkg/config.js");
        assert_eq!(result.status, ScanStatus::Skipped);
    }

    #[test]
    fn test_scanner_scan_dependencies_vulnerable() {
        let mut s = SecurityScanner::new();
        let package_json = r#"{
  "dependencies": {
    "lodash": "4.17.20",
    "express": "4.18.2"
  }
}"#;
        let result = s.scan_dependencies(package_json, "package.json");
        assert_eq!(result.status, ScanStatus::Completed);
        assert!(!result.findings.is_empty());
        let lodash_finding = result.findings.iter().find(|f| f.title.contains("lodash")).unwrap();
        assert!(lodash_finding.cve_id.is_some());
    }

    #[test]
    fn test_scanner_scan_dependencies_clean() {
        let mut s = SecurityScanner::new();
        let package_json = r#"{
  "dependencies": {
    "express": "4.18.2",
    "react": "18.2.0"
  }
}"#;
        let result = s.scan_dependencies(package_json, "package.json");
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_scanner_generate_report_empty() {
        let s = SecurityScanner::new();
        let report = s.generate_report();
        assert_eq!(report.total_findings, 0);
        assert!(!report.blocked);
        assert!(report.summary.contains("All clear"));
    }

    #[test]
    fn test_scanner_generate_report_with_findings() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        let report = s.generate_report();
        assert!(report.total_findings > 0);
        assert!(report.critical > 0);
        assert!(report.blocked);
    }

    #[test]
    fn test_scanner_suppress_finding() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        let finding_id = s.results[0].findings[0].id.clone();
        s.suppress_finding(&finding_id);
        assert!(s.results[0].findings[0].suppressed);
    }

    #[test]
    fn test_scanner_all_findings() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/a.rs");
        s.scan_content("postgres://user:pass@localhost:5432/db", "src/b.rs");
        let all = s.all_findings();
        assert!(all.len() >= 2);
    }

    #[test]
    fn test_scanner_should_block_pr_true() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        assert!(s.should_block_pr());
    }

    #[test]
    fn test_scanner_should_block_pr_false_clean() {
        let mut s = SecurityScanner::new();
        s.scan_content("fn main() {}", "src/main.rs");
        assert!(!s.should_block_pr());
    }

    #[test]
    fn test_scanner_to_markdown() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        let md = s.to_markdown();
        assert!(md.contains("# Security Scan Report"));
        assert!(md.contains("Critical"));
        assert!(md.contains("BLOCKED"));
    }

    #[test]
    fn test_scanner_to_markdown_clean() {
        let mut s = SecurityScanner::new();
        s.scan_content("fn main() {}", "src/main.rs");
        let md = s.to_markdown();
        assert!(md.contains("PASSED"));
    }

    #[test]
    fn test_scanner_max_findings_limit() {
        let mut s = SecurityScanner::new();
        s.config.max_findings = 1;
        // Content with multiple secrets on separate lines
        let content = "AKIAIOSFODNN7EXAMPLE1234567890\npassword = 'mysecretpassword'\npostgres://user:pass@localhost:5432/db\n";
        let result = s.scan_content(content, "src/config.rs");
        assert!(result.findings.len() <= 1);
    }

    #[test]
    fn test_scan_type_variants() {
        let types = vec![
            ScanType::SecretDetection,
            ScanType::DependencyVulnerability,
            ScanType::StaticAnalysis,
            ScanType::LicenseCompliance,
            ScanType::ConfigAudit,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_scan_status_variants() {
        let pending = ScanStatus::Pending;
        let running = ScanStatus::Running;
        let completed = ScanStatus::Completed;
        let failed = ScanStatus::Failed("err".to_string());
        let skipped = ScanStatus::Skipped;
        assert_eq!(pending, ScanStatus::Pending);
        assert_eq!(running, ScanStatus::Running);
        assert_eq!(completed, ScanStatus::Completed);
        assert_eq!(failed, ScanStatus::Failed("err".to_string()));
        assert_eq!(skipped, ScanStatus::Skipped);
    }

    #[test]
    fn test_dependency_vulnerability_struct() {
        let vuln = DependencyVulnerability {
            package: "lodash".to_string(),
            version: "4.17.20".to_string(),
            vulnerability: "CVE-2021-23337".to_string(),
            severity: ScanSeverity::High,
            fixed_version: Some("4.17.21".to_string()),
            description: "Prototype pollution".to_string(),
        };
        assert_eq!(vuln.package, "lodash");
        assert_eq!(vuln.severity, ScanSeverity::High);
    }

    #[test]
    fn test_report_blocked_on_critical() {
        let mut s = SecurityScanner::new();
        s.config.fail_on_severity = ScanSeverity::Critical;
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        let report = s.generate_report();
        assert!(report.blocked);
    }

    #[test]
    fn test_report_not_blocked_when_suppressed() {
        let mut s = SecurityScanner::new();
        s.scan_content("AKIAIOSFODNN7EXAMPLE1234567890", "src/config.rs");
        // Suppress all findings
        let ids: Vec<String> = s.all_findings().iter().map(|f| f.id.clone()).collect();
        for id in ids {
            s.suppress_finding(&id);
        }
        assert!(!s.should_block_pr());
    }

    #[test]
    fn test_scanner_multiple_scans() {
        let mut s = SecurityScanner::new();
        s.scan_content("clean code", "src/a.rs");
        s.scan_content("more clean code", "src/b.rs");
        s.scan_dependencies("{}", "package.json");
        assert_eq!(s.results.len(), 3);
    }
}
