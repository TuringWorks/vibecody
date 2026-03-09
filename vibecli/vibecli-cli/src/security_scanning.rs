//! Security scanning in agent flow — inline security analysis during agent execution.
//!
//! Closes P2 Gap 13: Agent scans code for vulnerabilities as it generates/edits.

// ---------------------------------------------------------------------------
// Security finding types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }

    pub fn score(&self) -> u32 {
        match self {
            Severity::Critical => 10,
            Severity::High => 8,
            Severity::Medium => 5,
            Severity::Low => 2,
            Severity::Info => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VulnerabilityClass {
    SqlInjection,
    Xss,
    CommandInjection,
    PathTraversal,
    HardcodedSecret,
    InsecureDeserialization,
    Ssrf,
    OpenRedirect,
    WeakCrypto,
    MissingAuth,
    BufferOverflow,
    RaceCondition,
    Custom(String),
}

impl VulnerabilityClass {
    pub fn as_str(&self) -> &str {
        match self {
            VulnerabilityClass::SqlInjection => "sql_injection",
            VulnerabilityClass::Xss => "xss",
            VulnerabilityClass::CommandInjection => "command_injection",
            VulnerabilityClass::PathTraversal => "path_traversal",
            VulnerabilityClass::HardcodedSecret => "hardcoded_secret",
            VulnerabilityClass::InsecureDeserialization => "insecure_deserialization",
            VulnerabilityClass::Ssrf => "ssrf",
            VulnerabilityClass::OpenRedirect => "open_redirect",
            VulnerabilityClass::WeakCrypto => "weak_crypto",
            VulnerabilityClass::MissingAuth => "missing_auth",
            VulnerabilityClass::BufferOverflow => "buffer_overflow",
            VulnerabilityClass::RaceCondition => "race_condition",
            VulnerabilityClass::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub severity: Severity,
    pub vuln_class: VulnerabilityClass,
    pub description: String,
    pub suggestion: Option<String>,
    pub cwe: Option<String>,
    pub suppressed: bool,
}

// ---------------------------------------------------------------------------
// Pattern-based scanner
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScanPattern {
    pub pattern: String,
    pub vuln_class: VulnerabilityClass,
    pub severity: Severity,
    pub description: String,
    pub languages: Vec<String>,
}

pub struct SecurityScanner {
    patterns: Vec<ScanPattern>,
    findings: Vec<SecurityFinding>,
    finding_counter: u64,
    suppressions: Vec<String>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        Self {
            patterns: default_patterns(),
            findings: Vec::new(),
            finding_counter: 0,
            suppressions: Vec::new(),
        }
    }

    pub fn scan_content(&mut self, file: &str, content: &str) -> Vec<String> {
        let mut new_ids = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            // Check suppression comment
            if line.contains("// nosec") || line.contains("# nosec") || line.contains("// NOSONAR") {
                continue;
            }
            for pat in &self.patterns {
                if line_lower.contains(&pat.pattern.to_lowercase()) {
                    self.finding_counter += 1;
                    let id = format!("finding-{}", self.finding_counter);
                    let finding = SecurityFinding {
                        id: id.clone(),
                        file: file.to_string(),
                        line: line_num + 1,
                        severity: pat.severity.clone(),
                        vuln_class: pat.vuln_class.clone(),
                        description: pat.description.clone(),
                        suggestion: None,
                        cwe: None,
                        suppressed: self.suppressions.contains(&pat.pattern),
                    };
                    self.findings.push(finding);
                    new_ids.push(id);
                }
            }
        }
        new_ids
    }

    pub fn scan_diff(&mut self, file: &str, added_lines: &[(usize, &str)]) -> Vec<String> {
        let mut new_ids = Vec::new();
        for (line_num, line) in added_lines {
            let line_lower = line.to_lowercase();
            if line.contains("// nosec") || line.contains("# nosec") {
                continue;
            }
            for pat in &self.patterns {
                if line_lower.contains(&pat.pattern.to_lowercase()) {
                    self.finding_counter += 1;
                    let id = format!("finding-{}", self.finding_counter);
                    let finding = SecurityFinding {
                        id: id.clone(),
                        file: file.to_string(),
                        line: *line_num,
                        severity: pat.severity.clone(),
                        vuln_class: pat.vuln_class.clone(),
                        description: pat.description.clone(),
                        suggestion: None,
                        cwe: None,
                        suppressed: false,
                    };
                    self.findings.push(finding);
                    new_ids.push(id);
                }
            }
        }
        new_ids
    }

    pub fn get_finding(&self, id: &str) -> Option<&SecurityFinding> {
        self.findings.iter().find(|f| f.id == id)
    }

    pub fn findings_for_file(&self, file: &str) -> Vec<&SecurityFinding> {
        self.findings.iter().filter(|f| f.file == file && !f.suppressed).collect()
    }

    pub fn findings_by_severity(&self, severity: &Severity) -> Vec<&SecurityFinding> {
        self.findings.iter().filter(|f| &f.severity == severity && !f.suppressed).collect()
    }

    pub fn suppress_pattern(&mut self, pattern: &str) {
        self.suppressions.push(pattern.to_string());
    }

    pub fn suppress_finding(&mut self, id: &str) -> bool {
        if let Some(f) = self.findings.iter_mut().find(|f| f.id == id) {
            f.suppressed = true;
            true
        } else {
            false
        }
    }

    pub fn has_blocking_findings(&self, min_severity: &Severity) -> bool {
        self.findings.iter().any(|f| !f.suppressed && f.severity.score() >= min_severity.score())
    }

    pub fn summary(&self) -> ScanSummary {
        let active: Vec<_> = self.findings.iter().filter(|f| !f.suppressed).collect();
        ScanSummary {
            total: self.findings.len(),
            active: active.len(),
            critical: active.iter().filter(|f| f.severity == Severity::Critical).count(),
            high: active.iter().filter(|f| f.severity == Severity::High).count(),
            medium: active.iter().filter(|f| f.severity == Severity::Medium).count(),
            low: active.iter().filter(|f| f.severity == Severity::Low).count(),
            info: active.iter().filter(|f| f.severity == Severity::Info).count(),
            suppressed: self.findings.len() - active.len(),
        }
    }

    pub fn add_pattern(&mut self, pattern: ScanPattern) {
        self.patterns.push(pattern);
    }

    pub fn total_findings(&self) -> usize {
        self.findings.len()
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub total: usize,
    pub active: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub suppressed: usize,
}

fn default_patterns() -> Vec<ScanPattern> {
    vec![
        ScanPattern {
            pattern: "eval(".to_string(),
            vuln_class: VulnerabilityClass::CommandInjection,
            severity: Severity::High,
            description: "Use of eval() can lead to code injection".to_string(),
            languages: vec!["javascript".into(), "python".into()],
        },
        ScanPattern {
            pattern: "exec(".to_string(),
            vuln_class: VulnerabilityClass::CommandInjection,
            severity: Severity::High,
            description: "Use of exec() can lead to command injection".to_string(),
            languages: vec!["python".into()],
        },
        ScanPattern {
            pattern: "password =".to_string(),
            vuln_class: VulnerabilityClass::HardcodedSecret,
            severity: Severity::Medium,
            description: "Possible hardcoded password".to_string(),
            languages: vec![],
        },
        ScanPattern {
            pattern: "api_key =".to_string(),
            vuln_class: VulnerabilityClass::HardcodedSecret,
            severity: Severity::Medium,
            description: "Possible hardcoded API key".to_string(),
            languages: vec![],
        },
        ScanPattern {
            pattern: "innerHTML".to_string(),
            vuln_class: VulnerabilityClass::Xss,
            severity: Severity::Medium,
            description: "innerHTML can lead to XSS if used with untrusted data".to_string(),
            languages: vec!["javascript".into(), "typescript".into()],
        },
        ScanPattern {
            pattern: "md5(".to_string(),
            vuln_class: VulnerabilityClass::WeakCrypto,
            severity: Severity::Low,
            description: "MD5 is cryptographically weak".to_string(),
            languages: vec![],
        },
        ScanPattern {
            pattern: "SELECT * FROM".to_string(),
            vuln_class: VulnerabilityClass::SqlInjection,
            severity: Severity::Medium,
            description: "Potential SQL injection if using string concatenation".to_string(),
            languages: vec![],
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity() {
        assert_eq!(Severity::Critical.as_str(), "critical");
        assert_eq!(Severity::Critical.score(), 10);
        assert_eq!(Severity::Low.score(), 2);
        assert_eq!(Severity::Info.score(), 0);
    }

    #[test]
    fn test_vuln_class() {
        assert_eq!(VulnerabilityClass::SqlInjection.as_str(), "sql_injection");
        assert_eq!(VulnerabilityClass::Custom("x".into()).as_str(), "x");
    }

    #[test]
    fn test_scan_content_finds_eval() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("app.js", "const x = eval(userInput);");
        assert!(!ids.is_empty());
        let finding = scanner.get_finding(&ids[0]).unwrap();
        assert_eq!(finding.severity, Severity::High);
        assert_eq!(finding.vuln_class, VulnerabilityClass::CommandInjection);
    }

    #[test]
    fn test_scan_content_finds_password() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("config.py", "password = 'secret123'");
        assert!(!ids.is_empty());
        let finding = scanner.get_finding(&ids[0]).unwrap();
        assert_eq!(finding.vuln_class, VulnerabilityClass::HardcodedSecret);
    }

    #[test]
    fn test_scan_nosec_suppression() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("app.js", "eval(x); // nosec");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_scan_diff() {
        let mut scanner = SecurityScanner::new();
        let lines = vec![(10, "element.innerHTML = data;")];
        let ids = scanner.scan_diff("page.js", &lines);
        assert!(!ids.is_empty());
    }

    #[test]
    fn test_findings_for_file() {
        let mut scanner = SecurityScanner::new();
        scanner.scan_content("a.js", "eval(x)");
        scanner.scan_content("b.js", "eval(y)");
        assert_eq!(scanner.findings_for_file("a.js").len(), 1);
    }

    #[test]
    fn test_findings_by_severity() {
        let mut scanner = SecurityScanner::new();
        scanner.scan_content("a.js", "eval(x)");
        scanner.scan_content("b.py", "password = 'abc'");
        assert!(!scanner.findings_by_severity(&Severity::High).is_empty());
        assert!(!scanner.findings_by_severity(&Severity::Medium).is_empty());
    }

    #[test]
    fn test_suppress_finding() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("a.js", "eval(x)");
        assert!(scanner.suppress_finding(&ids[0]));
        assert_eq!(scanner.findings_for_file("a.js").len(), 0);
    }

    #[test]
    fn test_has_blocking_findings() {
        let mut scanner = SecurityScanner::new();
        scanner.scan_content("a.js", "eval(x)");
        assert!(scanner.has_blocking_findings(&Severity::High));
        assert!(scanner.has_blocking_findings(&Severity::Medium));
        assert!(!scanner.has_blocking_findings(&Severity::Critical));
    }

    #[test]
    fn test_summary() {
        let mut scanner = SecurityScanner::new();
        scanner.scan_content("a.js", "eval(x)");
        scanner.scan_content("b.py", "password = 'abc'");
        let s = scanner.summary();
        assert!(s.total >= 2);
        assert_eq!(s.suppressed, 0);
    }

    #[test]
    fn test_summary_with_suppressed() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("a.js", "eval(x)");
        scanner.suppress_finding(&ids[0]);
        let s = scanner.summary();
        assert!(s.suppressed > 0);
        assert!(s.active < s.total);
    }

    #[test]
    fn test_add_custom_pattern() {
        let mut scanner = SecurityScanner::new();
        scanner.add_pattern(ScanPattern {
            pattern: "DANGER_ZONE".to_string(),
            vuln_class: VulnerabilityClass::Custom("custom_danger".into()),
            severity: Severity::Critical,
            description: "Custom dangerous pattern".to_string(),
            languages: vec![],
        });
        let ids = scanner.scan_content("test.rs", "let x = DANGER_ZONE;");
        assert!(!ids.is_empty());
    }

    #[test]
    fn test_clean_content() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("clean.rs", "fn main() { println!(\"hello\"); }");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_sql_injection_detection() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("db.py", "query = \"SELECT * FROM users WHERE id=\" + user_id");
        assert!(!ids.is_empty());
    }

    #[test]
    fn test_md5_detection() {
        let mut scanner = SecurityScanner::new();
        let ids = scanner.scan_content("hash.py", "h = md5(data)");
        assert!(!ids.is_empty());
        assert_eq!(scanner.get_finding(&ids[0]).unwrap().vuln_class, VulnerabilityClass::WeakCrypto);
    }

    #[test]
    fn test_total_findings() {
        let mut scanner = SecurityScanner::new();
        scanner.scan_content("a.js", "eval(x)");
        assert!(scanner.total_findings() >= 1);
    }

    #[test]
    fn test_suppress_nonexistent() {
        let mut scanner = SecurityScanner::new();
        assert!(!scanner.suppress_finding("fake-id"));
    }
}
