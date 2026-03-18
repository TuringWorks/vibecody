use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum QaStatus {
    Pending,
    InProgress,
    Passed,
    Failed,
    PassedWithWarnings,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QaCategory {
    Compilation,
    UnitTests,
    IntegrationTests,
    Security,
    Performance,
    CodeStyle,
    Documentation,
    TypeSafety,
    ErrorHandling,
    DependencyCheck,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QaAgentType {
    CompileChecker,
    TestRunner,
    SecurityAuditor,
    StyleEnforcer,
    DocValidator,
    PerformanceAnalyzer,
    DependencyAuditor,
    IntegrationTester,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QaRecommendation {
    Approve,
    ApproveWithWarnings(usize),
    RequestChanges(usize),
    Reject(String),
}

// === Core Structures ===

#[derive(Debug, Clone)]
pub struct QaFinding {
    pub id: String,
    pub category: QaCategory,
    pub severity: Severity,
    pub file_path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
    pub suggestion: Option<String>,
    pub agent_id: String,
    pub auto_fixable: bool,
    pub resolved: bool,
}

#[derive(Debug)]
pub struct QaAgent {
    pub id: String,
    pub agent_type: QaAgentType,
    pub status: QaStatus,
    pub findings: Vec<QaFinding>,
    pub files_reviewed: Vec<PathBuf>,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub pass_rate: f64,
}

#[derive(Debug)]
pub struct QaRound {
    pub round_number: usize,
    pub agents: Vec<QaAgent>,
    pub status: QaStatus,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub total_findings: usize,
    pub critical_findings: usize,
    pub findings_resolved: usize,
}

#[derive(Debug)]
pub struct CrossValidationResult {
    pub agent_a_id: String,
    pub agent_b_id: String,
    pub agreements: Vec<String>,
    pub disagreements: Vec<String>,
    pub confidence_score: f64,
}

#[derive(Debug)]
pub struct QaReport {
    pub run_id: String,
    pub rounds: Vec<QaRound>,
    pub cross_validations: Vec<CrossValidationResult>,
    pub total_findings: usize,
    pub critical_unresolved: usize,
    pub overall_score: f64,
    pub recommendation: QaRecommendation,
    pub generated_at: SystemTime,
}

#[derive(Debug)]
pub struct QaConfig {
    pub max_rounds: usize,
    pub min_pass_score: f64,
    pub require_zero_critical: bool,
    pub cross_validate: bool,
    pub auto_fix_enabled: bool,
    pub categories_enabled: Vec<QaCategory>,
    pub severity_threshold: Severity,
}

#[derive(Debug)]
pub struct QaPipeline {
    pub config: QaConfig,
    pub reports: Vec<QaReport>,
    pub active_round: Option<QaRound>,
}

// === Implementations ===

static FINDING_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_finding_id() -> String {
    let id = FINDING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    format!("QF-{:06}", id)
}

impl QaFinding {
    pub fn new(
        category: QaCategory,
        severity: Severity,
        file: PathBuf,
        message: &str,
        agent_id: &str,
    ) -> Self {
        Self {
            id: next_finding_id(),
            category,
            severity,
            file_path: file,
            line: None,
            message: message.to_string(),
            suggestion: None,
            agent_id: agent_id.to_string(),
            auto_fixable: false,
            resolved: false,
        }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    pub fn mark_auto_fixable(mut self) -> Self {
        self.auto_fixable = true;
        self
    }

    pub fn resolve(&mut self) {
        self.resolved = true;
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self.severity, Severity::Critical | Severity::High)
    }

    pub fn severity_score(&self) -> u32 {
        match self.severity {
            Severity::Critical => 10,
            Severity::High => 7,
            Severity::Medium => 4,
            Severity::Low => 2,
            Severity::Info => 0,
        }
    }
}

impl QaAgentType {
    fn label(&self) -> &str {
        match self {
            QaAgentType::CompileChecker => "compile-checker",
            QaAgentType::TestRunner => "test-runner",
            QaAgentType::SecurityAuditor => "security-auditor",
            QaAgentType::StyleEnforcer => "style-enforcer",
            QaAgentType::DocValidator => "doc-validator",
            QaAgentType::PerformanceAnalyzer => "performance-analyzer",
            QaAgentType::DependencyAuditor => "dependency-auditor",
            QaAgentType::IntegrationTester => "integration-tester",
        }
    }
}

impl QaAgent {
    pub fn new(agent_type: QaAgentType) -> Self {
        let id = format!("agent-{}", agent_type.label());
        Self {
            id,
            agent_type,
            status: QaStatus::Pending,
            findings: Vec::new(),
            files_reviewed: Vec::new(),
            started_at: None,
            completed_at: None,
            pass_rate: 100.0,
        }
    }

    pub fn start(&mut self) {
        self.status = QaStatus::InProgress;
        self.started_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(SystemTime::now());
        self.calculate_pass_rate();
        if self.critical_count() > 0 {
            self.status = QaStatus::Failed;
        } else if self.blocking_count() > 0 {
            self.status = QaStatus::PassedWithWarnings;
        } else {
            self.status = QaStatus::Passed;
        }
    }

    pub fn add_finding(&mut self, finding: QaFinding) {
        self.findings.push(finding);
    }

    pub fn review_file(&mut self, path: PathBuf) {
        if !self.files_reviewed.contains(&path) {
            self.files_reviewed.push(path);
        }
    }

    pub fn findings_by_severity(&self, severity: &Severity) -> Vec<&QaFinding> {
        self.findings
            .iter()
            .filter(|f| &f.severity == severity)
            .collect()
    }

    pub fn findings_by_category(&self, category: &QaCategory) -> Vec<&QaFinding> {
        self.findings
            .iter()
            .filter(|f| &f.category == category)
            .collect()
    }

    pub fn critical_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Critical && !f.resolved)
            .count()
    }

    pub fn blocking_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.is_blocking() && !f.resolved)
            .count()
    }

    pub fn auto_fixable_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.auto_fixable && !f.resolved)
            .count()
    }

    pub fn calculate_pass_rate(&mut self) {
        if self.files_reviewed.is_empty() {
            self.pass_rate = 100.0;
            return;
        }
        let total = self.files_reviewed.len() as f64;
        let files_with_blocking: usize = self
            .files_reviewed
            .iter()
            .filter(|path| {
                self.findings
                    .iter()
                    .any(|f| &f.file_path == *path && f.is_blocking() && !f.resolved)
            })
            .count();
        let passed = total - files_with_blocking as f64;
        self.pass_rate = (passed / total) * 100.0;
    }

    pub fn elapsed(&self) -> Duration {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => end.duration_since(start).unwrap_or(Duration::ZERO),
            (Some(start), None) => SystemTime::now()
                .duration_since(start)
                .unwrap_or(Duration::ZERO),
            _ => Duration::ZERO,
        }
    }
}

impl QaRound {
    pub fn new(round_number: usize) -> Self {
        Self {
            round_number,
            agents: Vec::new(),
            status: QaStatus::Pending,
            started_at: None,
            completed_at: None,
            total_findings: 0,
            critical_findings: 0,
            findings_resolved: 0,
        }
    }

    pub fn add_agent(&mut self, agent: QaAgent) {
        self.agents.push(agent);
    }

    pub fn start(&mut self) {
        self.status = QaStatus::InProgress;
        self.started_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.update_counts();
        self.completed_at = Some(SystemTime::now());
        if self.critical_unresolved().is_empty() && self.overall_pass_rate() >= 80.0 {
            if self.unresolved_findings().is_empty() {
                self.status = QaStatus::Passed;
            } else {
                self.status = QaStatus::PassedWithWarnings;
            }
        } else {
            self.status = QaStatus::Failed;
        }
    }

    pub fn all_findings(&self) -> Vec<&QaFinding> {
        self.agents.iter().flat_map(|a| a.findings.iter()).collect()
    }

    pub fn unresolved_findings(&self) -> Vec<&QaFinding> {
        self.agents
            .iter()
            .flat_map(|a| a.findings.iter())
            .filter(|f| !f.resolved)
            .collect()
    }

    pub fn critical_unresolved(&self) -> Vec<&QaFinding> {
        self.agents
            .iter()
            .flat_map(|a| a.findings.iter())
            .filter(|f| f.severity == Severity::Critical && !f.resolved)
            .collect()
    }

    pub fn overall_pass_rate(&self) -> f64 {
        if self.agents.is_empty() {
            return 100.0;
        }
        let sum: f64 = self.agents.iter().map(|a| a.pass_rate).sum();
        sum / self.agents.len() as f64
    }

    pub fn needs_another_round(&self, min_score: f64, require_zero_critical: bool) -> bool {
        let pass_rate = self.overall_pass_rate();
        if pass_rate < min_score {
            return true;
        }
        if require_zero_critical && !self.critical_unresolved().is_empty() {
            return true;
        }
        false
    }

    pub fn update_counts(&mut self) {
        let all: Vec<&QaFinding> = self.agents.iter().flat_map(|a| a.findings.iter()).collect();
        self.total_findings = all.len();
        self.critical_findings = all
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        self.findings_resolved = all.iter().filter(|f| f.resolved).count();
    }

    pub fn resolve_finding(&mut self, finding_id: &str) -> bool {
        for agent in &mut self.agents {
            for finding in &mut agent.findings {
                if finding.id == finding_id {
                    finding.resolve();
                    return true;
                }
            }
        }
        false
    }
}

impl CrossValidationResult {
    pub fn new(agent_a: &str, agent_b: &str) -> Self {
        Self {
            agent_a_id: agent_a.to_string(),
            agent_b_id: agent_b.to_string(),
            agreements: Vec::new(),
            disagreements: Vec::new(),
            confidence_score: 0.0,
        }
    }

    pub fn add_agreement(&mut self, finding_id: &str) {
        self.agreements.push(finding_id.to_string());
    }

    pub fn add_disagreement(&mut self, finding_id: &str) {
        self.disagreements.push(finding_id.to_string());
    }

    pub fn calculate_confidence(&mut self) {
        let total = self.agreements.len() + self.disagreements.len();
        if total == 0 {
            self.confidence_score = 1.0;
            return;
        }
        self.confidence_score = self.agreements.len() as f64 / total as f64;
    }

    pub fn agreement_rate(&self) -> f64 {
        let total = self.agreements.len() + self.disagreements.len();
        if total == 0 {
            return 1.0;
        }
        self.agreements.len() as f64 / total as f64
    }
}

impl QaConfig {
    pub fn default_config() -> Self {
        Self {
            max_rounds: 3,
            min_pass_score: 80.0,
            require_zero_critical: true,
            cross_validate: true,
            auto_fix_enabled: true,
            categories_enabled: vec![
                QaCategory::Compilation,
                QaCategory::UnitTests,
                QaCategory::IntegrationTests,
                QaCategory::Security,
                QaCategory::Performance,
                QaCategory::CodeStyle,
                QaCategory::Documentation,
                QaCategory::TypeSafety,
                QaCategory::ErrorHandling,
                QaCategory::DependencyCheck,
            ],
            severity_threshold: Severity::Low,
        }
    }

    pub fn strict() -> Self {
        Self {
            max_rounds: 5,
            min_pass_score: 95.0,
            require_zero_critical: true,
            cross_validate: true,
            auto_fix_enabled: true,
            categories_enabled: vec![
                QaCategory::Compilation,
                QaCategory::UnitTests,
                QaCategory::IntegrationTests,
                QaCategory::Security,
                QaCategory::Performance,
                QaCategory::CodeStyle,
                QaCategory::Documentation,
                QaCategory::TypeSafety,
                QaCategory::ErrorHandling,
                QaCategory::DependencyCheck,
            ],
            severity_threshold: Severity::Info,
        }
    }

    pub fn permissive() -> Self {
        Self {
            max_rounds: 2,
            min_pass_score: 60.0,
            require_zero_critical: false,
            cross_validate: false,
            auto_fix_enabled: false,
            categories_enabled: vec![
                QaCategory::Compilation,
                QaCategory::UnitTests,
                QaCategory::Security,
            ],
            severity_threshold: Severity::High,
        }
    }
}

impl QaPipeline {
    pub fn new() -> Self {
        Self {
            config: QaConfig::default_config(),
            reports: Vec::new(),
            active_round: None,
        }
    }

    pub fn with_config(config: QaConfig) -> Self {
        Self {
            config,
            reports: Vec::new(),
            active_round: None,
        }
    }

    pub fn create_round(&mut self) -> &mut QaRound {
        let round_number = match &self.active_round {
            Some(r) => r.round_number + 1,
            None => 1,
        };
        self.active_round = Some(QaRound::new(round_number));
        self.active_round.as_mut().expect("just created active_round")
    }

    pub fn run_cross_validation(&self, round: &QaRound) -> Vec<CrossValidationResult> {
        let mut results = Vec::new();
        let agents = &round.agents;
        for i in 0..agents.len() {
            for j in (i + 1)..agents.len() {
                let a = &agents[i];
                let b = &agents[j];
                let mut cv = CrossValidationResult::new(&a.id, &b.id);

                // Collect file paths flagged by each agent
                let a_files: HashMap<&PathBuf, Vec<&QaFinding>> = {
                    let mut map: HashMap<&PathBuf, Vec<&QaFinding>> = HashMap::new();
                    for f in &a.findings {
                        map.entry(&f.file_path).or_default().push(f);
                    }
                    map
                };
                let b_files: HashMap<&PathBuf, Vec<&QaFinding>> = {
                    let mut map: HashMap<&PathBuf, Vec<&QaFinding>> = HashMap::new();
                    for f in &b.findings {
                        map.entry(&f.file_path).or_default().push(f);
                    }
                    map
                };

                // Agreement: both agents flagged the same file
                for (path, a_findings) in &a_files {
                    if let Some(b_findings) = b_files.get(path) {
                        // Both flagged this file — each shared finding is an agreement
                        for af in a_findings {
                            cv.add_agreement(&af.id);
                        }
                        for bf in b_findings {
                            cv.add_agreement(&bf.id);
                        }
                    } else {
                        // Only agent A flagged this file
                        for af in a_findings {
                            cv.add_disagreement(&af.id);
                        }
                    }
                }
                // Files only agent B flagged
                for (path, b_findings) in &b_files {
                    if !a_files.contains_key(path) {
                        for bf in b_findings {
                            cv.add_disagreement(&bf.id);
                        }
                    }
                }

                cv.calculate_confidence();
                results.push(cv);
            }
        }
        results
    }

    pub fn generate_report(&self, run_id: &str) -> QaReport {
        let round = self.active_round.as_ref();
        let (total_findings, critical_unresolved, overall_score) = match round {
            Some(r) => {
                let total = r.all_findings().len();
                let critical = r.critical_unresolved().len();
                let score = self.calculate_overall_score(r);
                (total, critical, score)
            }
            None => (0, 0, 100.0),
        };

        let recommendation = self.make_recommendation(overall_score, critical_unresolved);
        let cross_validations = match round {
            Some(r) if self.config.cross_validate => self.run_cross_validation(r),
            _ => Vec::new(),
        };

        QaReport {
            run_id: run_id.to_string(),
            rounds: Vec::new(), // rounds are consumed separately
            cross_validations,
            total_findings,
            critical_unresolved,
            overall_score,
            recommendation,
            generated_at: SystemTime::now(),
        }
    }

    pub fn calculate_overall_score(&self, round: &QaRound) -> f64 {
        let pass_rate = round.overall_pass_rate();
        let all = round.all_findings();
        if all.is_empty() {
            return pass_rate;
        }
        let total_severity: u32 = all.iter().filter(|f| !f.resolved).map(|f| f.severity_score()).sum();
        let max_possible = all.len() as u32 * 10; // worst case: all critical
        let severity_penalty = if max_possible > 0 {
            (total_severity as f64 / max_possible as f64) * 50.0
        } else {
            0.0
        };
        let score = pass_rate - severity_penalty;
        score.clamp(0.0, 100.0)
    }

    pub fn make_recommendation(&self, score: f64, critical_count: usize) -> QaRecommendation {
        if self.config.require_zero_critical && critical_count > 0 {
            return QaRecommendation::Reject(format!(
                "{} critical findings must be resolved",
                critical_count
            ));
        }
        if score >= self.config.min_pass_score {
            if critical_count == 0 {
                QaRecommendation::Approve
            } else {
                QaRecommendation::ApproveWithWarnings(critical_count)
            }
        } else if score >= self.config.min_pass_score * 0.75 {
            QaRecommendation::RequestChanges(
                ((self.config.min_pass_score - score).ceil()) as usize,
            )
        } else {
            QaRecommendation::Reject(format!("Score {:.1} is below minimum threshold", score))
        }
    }

    pub fn should_continue(&self, round: &QaRound) -> bool {
        if round.round_number >= self.config.max_rounds {
            return false;
        }
        round.needs_another_round(self.config.min_pass_score, self.config.require_zero_critical)
    }

    pub fn spawn_standard_agents(&self) -> Vec<QaAgent> {
        vec![
            QaAgent::new(QaAgentType::CompileChecker),
            QaAgent::new(QaAgentType::TestRunner),
            QaAgent::new(QaAgentType::SecurityAuditor),
            QaAgent::new(QaAgentType::StyleEnforcer),
            QaAgent::new(QaAgentType::DocValidator),
            QaAgent::new(QaAgentType::PerformanceAnalyzer),
            QaAgent::new(QaAgentType::DependencyAuditor),
            QaAgent::new(QaAgentType::IntegrationTester),
        ]
    }

    pub fn history(&self) -> &[QaReport] {
        &self.reports
    }

    pub fn best_score(&self) -> Option<f64> {
        self.reports
            .iter()
            .map(|r| r.overall_score)
            .fold(None, |best, s| match best {
                None => Some(s),
                Some(b) if s > b => Some(s),
                _ => best,
            })
    }

    pub fn worst_score(&self) -> Option<f64> {
        self.reports
            .iter()
            .map(|r| r.overall_score)
            .fold(None, |worst, s| match worst {
                None => Some(s),
                Some(w) if s < w => Some(s),
                _ => worst,
            })
    }
}

impl QaReport {
    pub fn is_approved(&self) -> bool {
        matches!(
            self.recommendation,
            QaRecommendation::Approve | QaRecommendation::ApproveWithWarnings(_)
        )
    }

    pub fn summary_line(&self) -> String {
        let status = match &self.recommendation {
            QaRecommendation::Approve => "APPROVED".to_string(),
            QaRecommendation::ApproveWithWarnings(n) => {
                format!("APPROVED WITH {} WARNING(S)", n)
            }
            QaRecommendation::RequestChanges(n) => {
                format!("CHANGES REQUESTED (gap: {})", n)
            }
            QaRecommendation::Reject(reason) => format!("REJECTED: {}", reason),
        };
        format!(
            "[{}] Score: {:.1}/100 | Findings: {} | Critical unresolved: {}",
            status, self.overall_score, self.total_findings, self.critical_unresolved
        )
    }

    pub fn findings_by_file(&self) -> HashMap<PathBuf, Vec<&QaFinding>> {
        let mut map: HashMap<PathBuf, Vec<&QaFinding>> = HashMap::new();
        for round in &self.rounds {
            for agent in &round.agents {
                for finding in &agent.findings {
                    map.entry(finding.file_path.clone())
                        .or_default()
                        .push(finding);
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(severity: Severity, agent_id: &str) -> QaFinding {
        QaFinding::new(
            QaCategory::Compilation,
            severity,
            PathBuf::from("src/main.rs"),
            "test finding",
            agent_id,
        )
    }

    // --- QaFinding tests ---

    #[test]
    fn test_finding_new() {
        let f = QaFinding::new(
            QaCategory::Security,
            Severity::High,
            PathBuf::from("lib.rs"),
            "SQL injection risk",
            "agent-1",
        );
        assert_eq!(f.category, QaCategory::Security);
        assert_eq!(f.severity, Severity::High);
        assert_eq!(f.message, "SQL injection risk");
        assert_eq!(f.agent_id, "agent-1");
        assert!(!f.resolved);
        assert!(!f.auto_fixable);
        assert!(f.line.is_none());
        assert!(f.suggestion.is_none());
    }

    #[test]
    fn test_finding_with_line() {
        let f = make_finding(Severity::Low, "a1").with_line(42);
        assert_eq!(f.line, Some(42));
    }

    #[test]
    fn test_finding_with_suggestion() {
        let f = make_finding(Severity::Low, "a1").with_suggestion("Use parameterized queries");
        assert_eq!(f.suggestion.as_deref(), Some("Use parameterized queries"));
    }

    #[test]
    fn test_finding_mark_auto_fixable() {
        let f = make_finding(Severity::Low, "a1").mark_auto_fixable();
        assert!(f.auto_fixable);
    }

    #[test]
    fn test_finding_resolve() {
        let mut f = make_finding(Severity::Critical, "a1");
        assert!(!f.resolved);
        f.resolve();
        assert!(f.resolved);
    }

    #[test]
    fn test_finding_is_blocking_critical() {
        let f = make_finding(Severity::Critical, "a1");
        assert!(f.is_blocking());
    }

    #[test]
    fn test_finding_is_blocking_high() {
        let f = make_finding(Severity::High, "a1");
        assert!(f.is_blocking());
    }

    #[test]
    fn test_finding_not_blocking_medium() {
        let f = make_finding(Severity::Medium, "a1");
        assert!(!f.is_blocking());
    }

    #[test]
    fn test_finding_not_blocking_low() {
        let f = make_finding(Severity::Low, "a1");
        assert!(!f.is_blocking());
    }

    #[test]
    fn test_finding_not_blocking_info() {
        let f = make_finding(Severity::Info, "a1");
        assert!(!f.is_blocking());
    }

    #[test]
    fn test_severity_score_critical() {
        assert_eq!(make_finding(Severity::Critical, "a").severity_score(), 10);
    }

    #[test]
    fn test_severity_score_high() {
        assert_eq!(make_finding(Severity::High, "a").severity_score(), 7);
    }

    #[test]
    fn test_severity_score_medium() {
        assert_eq!(make_finding(Severity::Medium, "a").severity_score(), 4);
    }

    #[test]
    fn test_severity_score_low() {
        assert_eq!(make_finding(Severity::Low, "a").severity_score(), 2);
    }

    #[test]
    fn test_severity_score_info() {
        assert_eq!(make_finding(Severity::Info, "a").severity_score(), 0);
    }

    #[test]
    fn test_finding_builder_chain() {
        let f = make_finding(Severity::Medium, "a1")
            .with_line(10)
            .with_suggestion("fix it")
            .mark_auto_fixable();
        assert_eq!(f.line, Some(10));
        assert_eq!(f.suggestion.as_deref(), Some("fix it"));
        assert!(f.auto_fixable);
    }

    // --- QaAgent tests ---

    #[test]
    fn test_agent_new() {
        let a = QaAgent::new(QaAgentType::CompileChecker);
        assert_eq!(a.id, "agent-compile-checker");
        assert_eq!(a.status, QaStatus::Pending);
        assert!(a.findings.is_empty());
        assert_eq!(a.pass_rate, 100.0);
    }

    #[test]
    fn test_agent_start() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.start();
        assert_eq!(a.status, QaStatus::InProgress);
        assert!(a.started_at.is_some());
    }

    #[test]
    fn test_agent_complete_no_findings() {
        let mut a = QaAgent::new(QaAgentType::DocValidator);
        a.start();
        a.complete();
        assert_eq!(a.status, QaStatus::Passed);
        assert!(a.completed_at.is_some());
    }

    #[test]
    fn test_agent_complete_with_critical() {
        let mut a = QaAgent::new(QaAgentType::SecurityAuditor);
        a.start();
        a.review_file(PathBuf::from("src/main.rs"));
        a.add_finding(make_finding(Severity::Critical, &a.id.clone()));
        a.complete();
        assert_eq!(a.status, QaStatus::Failed);
    }

    #[test]
    fn test_agent_complete_with_high_only() {
        let mut a = QaAgent::new(QaAgentType::StyleEnforcer);
        a.start();
        a.review_file(PathBuf::from("src/main.rs"));
        a.add_finding(make_finding(Severity::High, &a.id.clone()));
        a.complete();
        assert_eq!(a.status, QaStatus::PassedWithWarnings);
    }

    #[test]
    fn test_agent_add_finding() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(make_finding(Severity::Low, "a1"));
        assert_eq!(a.findings.len(), 1);
    }

    #[test]
    fn test_agent_review_file_dedup() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.review_file(PathBuf::from("src/lib.rs"));
        a.review_file(PathBuf::from("src/lib.rs"));
        assert_eq!(a.files_reviewed.len(), 1);
    }

    #[test]
    fn test_agent_findings_by_severity() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(make_finding(Severity::Critical, "a1"));
        a.add_finding(make_finding(Severity::Low, "a1"));
        a.add_finding(make_finding(Severity::Critical, "a1"));
        assert_eq!(a.findings_by_severity(&Severity::Critical).len(), 2);
        assert_eq!(a.findings_by_severity(&Severity::Low).len(), 1);
        assert_eq!(a.findings_by_severity(&Severity::Medium).len(), 0);
    }

    #[test]
    fn test_agent_findings_by_category() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(QaFinding::new(
            QaCategory::Security,
            Severity::High,
            PathBuf::from("x.rs"),
            "sec issue",
            "a1",
        ));
        a.add_finding(make_finding(Severity::Low, "a1")); // Compilation
        assert_eq!(a.findings_by_category(&QaCategory::Security).len(), 1);
        assert_eq!(a.findings_by_category(&QaCategory::Compilation).len(), 1);
    }

    #[test]
    fn test_agent_critical_count() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(make_finding(Severity::Critical, "a1"));
        let mut f = make_finding(Severity::Critical, "a1");
        f.resolve();
        a.add_finding(f);
        assert_eq!(a.critical_count(), 1); // resolved one doesn't count
    }

    #[test]
    fn test_agent_blocking_count() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(make_finding(Severity::Critical, "a1"));
        a.add_finding(make_finding(Severity::High, "a1"));
        a.add_finding(make_finding(Severity::Medium, "a1"));
        assert_eq!(a.blocking_count(), 2);
    }

    #[test]
    fn test_agent_auto_fixable_count() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.add_finding(make_finding(Severity::Low, "a1").mark_auto_fixable());
        a.add_finding(make_finding(Severity::Low, "a1"));
        assert_eq!(a.auto_fixable_count(), 1);
    }

    #[test]
    fn test_agent_pass_rate_no_files() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.calculate_pass_rate();
        assert_eq!(a.pass_rate, 100.0);
    }

    #[test]
    fn test_agent_pass_rate_with_blocking() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.review_file(PathBuf::from("a.rs"));
        a.review_file(PathBuf::from("b.rs"));
        a.add_finding(make_finding(Severity::Critical, "a1")); // file: src/main.rs
        a.calculate_pass_rate();
        assert_eq!(a.pass_rate, 100.0); // finding is on src/main.rs which isn't in reviewed files
    }

    #[test]
    fn test_agent_pass_rate_with_matching_file() {
        let mut a = QaAgent::new(QaAgentType::TestRunner);
        a.review_file(PathBuf::from("src/main.rs"));
        a.review_file(PathBuf::from("src/lib.rs"));
        a.add_finding(make_finding(Severity::Critical, "a1")); // file: src/main.rs
        a.calculate_pass_rate();
        assert_eq!(a.pass_rate, 50.0);
    }

    #[test]
    fn test_agent_elapsed_not_started() {
        let a = QaAgent::new(QaAgentType::TestRunner);
        assert_eq!(a.elapsed(), Duration::ZERO);
    }

    // --- QaRound tests ---

    #[test]
    fn test_round_new() {
        let r = QaRound::new(1);
        assert_eq!(r.round_number, 1);
        assert_eq!(r.status, QaStatus::Pending);
        assert!(r.agents.is_empty());
    }

    #[test]
    fn test_round_add_agent() {
        let mut r = QaRound::new(1);
        r.add_agent(QaAgent::new(QaAgentType::CompileChecker));
        assert_eq!(r.agents.len(), 1);
    }

    #[test]
    fn test_round_start() {
        let mut r = QaRound::new(1);
        r.start();
        assert_eq!(r.status, QaStatus::InProgress);
        assert!(r.started_at.is_some());
    }

    #[test]
    fn test_round_complete_passed() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 100.0;
        r.add_agent(a);
        r.start();
        r.complete();
        assert_eq!(r.status, QaStatus::Passed);
    }

    #[test]
    fn test_round_complete_failed_critical() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::SecurityAuditor);
        a.review_file(PathBuf::from("src/main.rs"));
        a.add_finding(make_finding(Severity::Critical, "agent-security-auditor"));
        a.pass_rate = 0.0;
        r.add_agent(a);
        r.start();
        r.complete();
        assert_eq!(r.status, QaStatus::Failed);
    }

    #[test]
    fn test_round_all_findings() {
        let mut r = QaRound::new(1);
        let mut a1 = QaAgent::new(QaAgentType::CompileChecker);
        a1.add_finding(make_finding(Severity::Low, "a1"));
        let mut a2 = QaAgent::new(QaAgentType::TestRunner);
        a2.add_finding(make_finding(Severity::Medium, "a2"));
        a2.add_finding(make_finding(Severity::High, "a2"));
        r.add_agent(a1);
        r.add_agent(a2);
        assert_eq!(r.all_findings().len(), 3);
    }

    #[test]
    fn test_round_unresolved_findings() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.add_finding(make_finding(Severity::Low, "a1"));
        let mut resolved = make_finding(Severity::High, "a1");
        resolved.resolve();
        a.add_finding(resolved);
        r.add_agent(a);
        assert_eq!(r.unresolved_findings().len(), 1);
    }

    #[test]
    fn test_round_critical_unresolved() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.add_finding(make_finding(Severity::Critical, "a1"));
        a.add_finding(make_finding(Severity::High, "a1"));
        r.add_agent(a);
        assert_eq!(r.critical_unresolved().len(), 1);
    }

    #[test]
    fn test_round_overall_pass_rate_empty() {
        let r = QaRound::new(1);
        assert_eq!(r.overall_pass_rate(), 100.0);
    }

    #[test]
    fn test_round_overall_pass_rate() {
        let mut r = QaRound::new(1);
        let mut a1 = QaAgent::new(QaAgentType::CompileChecker);
        a1.pass_rate = 80.0;
        let mut a2 = QaAgent::new(QaAgentType::TestRunner);
        a2.pass_rate = 60.0;
        r.add_agent(a1);
        r.add_agent(a2);
        assert_eq!(r.overall_pass_rate(), 70.0);
    }

    #[test]
    fn test_round_needs_another_round_low_score() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 50.0;
        r.add_agent(a);
        assert!(r.needs_another_round(80.0, false));
    }

    #[test]
    fn test_round_needs_another_round_critical() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 100.0;
        a.add_finding(make_finding(Severity::Critical, "a1"));
        r.add_agent(a);
        assert!(r.needs_another_round(80.0, true));
    }

    #[test]
    fn test_round_no_more_rounds_needed() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 95.0;
        r.add_agent(a);
        assert!(!r.needs_another_round(80.0, true));
    }

    #[test]
    fn test_round_update_counts() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.add_finding(make_finding(Severity::Critical, "a1"));
        a.add_finding(make_finding(Severity::Low, "a1"));
        let mut resolved = make_finding(Severity::Medium, "a1");
        resolved.resolve();
        a.add_finding(resolved);
        r.add_agent(a);
        r.update_counts();
        assert_eq!(r.total_findings, 3);
        assert_eq!(r.critical_findings, 1);
        assert_eq!(r.findings_resolved, 1);
    }

    #[test]
    fn test_round_resolve_finding() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        let f = make_finding(Severity::Critical, "a1");
        let fid = f.id.clone();
        a.add_finding(f);
        r.add_agent(a);
        assert!(r.resolve_finding(&fid));
        assert!(r.critical_unresolved().is_empty());
    }

    #[test]
    fn test_round_resolve_finding_not_found() {
        let mut r = QaRound::new(1);
        r.add_agent(QaAgent::new(QaAgentType::CompileChecker));
        assert!(!r.resolve_finding("nonexistent"));
    }

    // --- CrossValidationResult tests ---

    #[test]
    fn test_cross_validation_new() {
        let cv = CrossValidationResult::new("a", "b");
        assert_eq!(cv.agent_a_id, "a");
        assert_eq!(cv.agent_b_id, "b");
        assert!(cv.agreements.is_empty());
        assert!(cv.disagreements.is_empty());
    }

    #[test]
    fn test_cross_validation_add_agreement() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_agreement("f1");
        assert_eq!(cv.agreements.len(), 1);
        assert_eq!(cv.agreements[0], "f1");
    }

    #[test]
    fn test_cross_validation_add_disagreement() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_disagreement("f2");
        assert_eq!(cv.disagreements.len(), 1);
    }

    #[test]
    fn test_cross_validation_confidence_all_agree() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_agreement("f1");
        cv.add_agreement("f2");
        cv.calculate_confidence();
        assert_eq!(cv.confidence_score, 1.0);
    }

    #[test]
    fn test_cross_validation_confidence_all_disagree() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_disagreement("f1");
        cv.add_disagreement("f2");
        cv.calculate_confidence();
        assert_eq!(cv.confidence_score, 0.0);
    }

    #[test]
    fn test_cross_validation_confidence_mixed() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_agreement("f1");
        cv.add_disagreement("f2");
        cv.calculate_confidence();
        assert_eq!(cv.confidence_score, 0.5);
    }

    #[test]
    fn test_cross_validation_confidence_empty() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.calculate_confidence();
        assert_eq!(cv.confidence_score, 1.0);
    }

    #[test]
    fn test_cross_validation_agreement_rate() {
        let mut cv = CrossValidationResult::new("a", "b");
        cv.add_agreement("f1");
        cv.add_agreement("f2");
        cv.add_disagreement("f3");
        assert!((cv.agreement_rate() - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cross_validation_agreement_rate_empty() {
        let cv = CrossValidationResult::new("a", "b");
        assert_eq!(cv.agreement_rate(), 1.0);
    }

    // --- QaConfig tests ---

    #[test]
    fn test_config_default() {
        let c = QaConfig::default_config();
        assert_eq!(c.max_rounds, 3);
        assert_eq!(c.min_pass_score, 80.0);
        assert!(c.require_zero_critical);
        assert!(c.cross_validate);
        assert!(c.auto_fix_enabled);
        assert_eq!(c.categories_enabled.len(), 10);
        assert_eq!(c.severity_threshold, Severity::Low);
    }

    #[test]
    fn test_config_strict() {
        let c = QaConfig::strict();
        assert_eq!(c.max_rounds, 5);
        assert_eq!(c.min_pass_score, 95.0);
        assert!(c.require_zero_critical);
        assert_eq!(c.severity_threshold, Severity::Info);
    }

    #[test]
    fn test_config_permissive() {
        let c = QaConfig::permissive();
        assert_eq!(c.max_rounds, 2);
        assert_eq!(c.min_pass_score, 60.0);
        assert!(!c.require_zero_critical);
        assert!(!c.cross_validate);
        assert_eq!(c.categories_enabled.len(), 3);
        assert_eq!(c.severity_threshold, Severity::High);
    }

    // --- QaPipeline tests ---

    #[test]
    fn test_pipeline_new() {
        let p = QaPipeline::new();
        assert_eq!(p.config.max_rounds, 3);
        assert!(p.reports.is_empty());
        assert!(p.active_round.is_none());
    }

    #[test]
    fn test_pipeline_with_config() {
        let p = QaPipeline::with_config(QaConfig::strict());
        assert_eq!(p.config.min_pass_score, 95.0);
    }

    #[test]
    fn test_pipeline_create_round() {
        let mut p = QaPipeline::new();
        let r = p.create_round();
        assert_eq!(r.round_number, 1);
    }

    #[test]
    fn test_pipeline_create_multiple_rounds() {
        let mut p = QaPipeline::new();
        p.create_round();
        p.create_round();
        assert_eq!(p.active_round.as_ref().expect("active round").round_number, 2);
    }

    #[test]
    fn test_pipeline_spawn_standard_agents() {
        let p = QaPipeline::new();
        let agents = p.spawn_standard_agents();
        assert_eq!(agents.len(), 8);
        assert_eq!(agents[0].agent_type, QaAgentType::CompileChecker);
        assert_eq!(agents[7].agent_type, QaAgentType::IntegrationTester);
    }

    #[test]
    fn test_pipeline_should_continue_max_rounds() {
        let p = QaPipeline::new(); // max 3 rounds
        let mut r = QaRound::new(3);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 50.0; // low score, but max rounds reached
        r.add_agent(a);
        assert!(!p.should_continue(&r));
    }

    #[test]
    fn test_pipeline_should_continue_needs_improvement() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 50.0;
        r.add_agent(a);
        assert!(p.should_continue(&r));
    }

    #[test]
    fn test_pipeline_should_not_continue_passing() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 95.0;
        r.add_agent(a);
        assert!(!p.should_continue(&r));
    }

    #[test]
    fn test_pipeline_make_recommendation_approve() {
        let p = QaPipeline::new();
        let rec = p.make_recommendation(90.0, 0);
        assert_eq!(rec, QaRecommendation::Approve);
    }

    #[test]
    fn test_pipeline_make_recommendation_reject_critical() {
        let p = QaPipeline::new(); // require_zero_critical = true
        let rec = p.make_recommendation(95.0, 2);
        match rec {
            QaRecommendation::Reject(msg) => assert!(msg.contains("2 critical")),
            _ => panic!("expected Reject"),
        }
    }

    #[test]
    fn test_pipeline_make_recommendation_request_changes() {
        let mut p = QaPipeline::new();
        p.config.require_zero_critical = false;
        let rec = p.make_recommendation(65.0, 0); // below 80 but above 60
        match rec {
            QaRecommendation::RequestChanges(_) => {}
            _ => panic!("expected RequestChanges"),
        }
    }

    #[test]
    fn test_pipeline_make_recommendation_reject_low_score() {
        let mut p = QaPipeline::new();
        p.config.require_zero_critical = false;
        let rec = p.make_recommendation(30.0, 0);
        match rec {
            QaRecommendation::Reject(msg) => assert!(msg.contains("30.0")),
            _ => panic!("expected Reject"),
        }
    }

    #[test]
    fn test_pipeline_make_recommendation_approve_with_warnings() {
        let mut p = QaPipeline::new();
        p.config.require_zero_critical = false;
        let rec = p.make_recommendation(90.0, 1);
        assert_eq!(rec, QaRecommendation::ApproveWithWarnings(1));
    }

    #[test]
    fn test_pipeline_calculate_overall_score_no_findings() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 90.0;
        r.add_agent(a);
        assert_eq!(p.calculate_overall_score(&r), 90.0);
    }

    #[test]
    fn test_pipeline_calculate_overall_score_with_findings() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 100.0;
        a.add_finding(make_finding(Severity::Low, "a1")); // score=2, max=10
        r.add_agent(a);
        let score = p.calculate_overall_score(&r);
        // pass_rate=100, severity_penalty = (2/10)*50 = 10, score = 90
        assert_eq!(score, 90.0);
    }

    #[test]
    fn test_pipeline_generate_report() {
        let mut p = QaPipeline::new();
        p.create_round();
        let report = p.generate_report("run-1");
        assert_eq!(report.run_id, "run-1");
        assert_eq!(report.total_findings, 0);
        assert_eq!(report.overall_score, 100.0);
        assert_eq!(report.recommendation, QaRecommendation::Approve);
    }

    #[test]
    fn test_pipeline_generate_report_no_active_round() {
        let p = QaPipeline::new();
        let report = p.generate_report("run-x");
        assert_eq!(report.total_findings, 0);
        assert_eq!(report.overall_score, 100.0);
    }

    #[test]
    fn test_pipeline_history_empty() {
        let p = QaPipeline::new();
        assert!(p.history().is_empty());
    }

    #[test]
    fn test_pipeline_best_score_empty() {
        let p = QaPipeline::new();
        assert!(p.best_score().is_none());
    }

    #[test]
    fn test_pipeline_worst_score_empty() {
        let p = QaPipeline::new();
        assert!(p.worst_score().is_none());
    }

    #[test]
    fn test_pipeline_best_and_worst_score() {
        let mut p = QaPipeline::new();
        p.reports.push(QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 0,
            critical_unresolved: 0,
            overall_score: 85.0,
            recommendation: QaRecommendation::Approve,
            generated_at: SystemTime::now(),
        });
        p.reports.push(QaReport {
            run_id: "r2".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 2,
            critical_unresolved: 1,
            overall_score: 55.0,
            recommendation: QaRecommendation::RequestChanges(3),
            generated_at: SystemTime::now(),
        });
        assert_eq!(p.best_score(), Some(85.0));
        assert_eq!(p.worst_score(), Some(55.0));
    }

    #[test]
    fn test_pipeline_cross_validation_no_agents() {
        let p = QaPipeline::new();
        let r = QaRound::new(1);
        let results = p.run_cross_validation(&r);
        assert!(results.is_empty());
    }

    #[test]
    fn test_pipeline_cross_validation_single_agent() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        r.add_agent(QaAgent::new(QaAgentType::CompileChecker));
        let results = p.run_cross_validation(&r);
        assert!(results.is_empty()); // need at least 2 agents
    }

    #[test]
    fn test_pipeline_cross_validation_two_agents_same_file() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a1 = QaAgent::new(QaAgentType::CompileChecker);
        a1.add_finding(make_finding(Severity::Low, "agent-compile-checker"));
        let mut a2 = QaAgent::new(QaAgentType::SecurityAuditor);
        a2.add_finding(QaFinding::new(
            QaCategory::Security,
            Severity::High,
            PathBuf::from("src/main.rs"), // same file
            "issue",
            "agent-security-auditor",
        ));
        r.add_agent(a1);
        r.add_agent(a2);
        let results = p.run_cross_validation(&r);
        assert_eq!(results.len(), 1);
        assert!(!results[0].agreements.is_empty()); // both flagged src/main.rs
    }

    #[test]
    fn test_pipeline_cross_validation_two_agents_different_files() {
        let p = QaPipeline::new();
        let mut r = QaRound::new(1);
        let mut a1 = QaAgent::new(QaAgentType::CompileChecker);
        a1.add_finding(QaFinding::new(
            QaCategory::Compilation,
            Severity::Low,
            PathBuf::from("a.rs"),
            "issue",
            "agent-compile-checker",
        ));
        let mut a2 = QaAgent::new(QaAgentType::SecurityAuditor);
        a2.add_finding(QaFinding::new(
            QaCategory::Security,
            Severity::High,
            PathBuf::from("b.rs"),
            "issue",
            "agent-security-auditor",
        ));
        r.add_agent(a1);
        r.add_agent(a2);
        let results = p.run_cross_validation(&r);
        assert_eq!(results.len(), 1);
        assert!(results[0].agreements.is_empty());
        assert_eq!(results[0].disagreements.len(), 2); // each file is a disagreement
    }

    // --- QaReport tests ---

    #[test]
    fn test_report_is_approved() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 0,
            critical_unresolved: 0,
            overall_score: 95.0,
            recommendation: QaRecommendation::Approve,
            generated_at: SystemTime::now(),
        };
        assert!(report.is_approved());
    }

    #[test]
    fn test_report_is_approved_with_warnings() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 2,
            critical_unresolved: 0,
            overall_score: 85.0,
            recommendation: QaRecommendation::ApproveWithWarnings(2),
            generated_at: SystemTime::now(),
        };
        assert!(report.is_approved());
    }

    #[test]
    fn test_report_not_approved_reject() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 5,
            critical_unresolved: 3,
            overall_score: 40.0,
            recommendation: QaRecommendation::Reject("too many issues".to_string()),
            generated_at: SystemTime::now(),
        };
        assert!(!report.is_approved());
    }

    #[test]
    fn test_report_not_approved_request_changes() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 3,
            critical_unresolved: 0,
            overall_score: 70.0,
            recommendation: QaRecommendation::RequestChanges(10),
            generated_at: SystemTime::now(),
        };
        assert!(!report.is_approved());
    }

    #[test]
    fn test_report_summary_line_approved() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 0,
            critical_unresolved: 0,
            overall_score: 100.0,
            recommendation: QaRecommendation::Approve,
            generated_at: SystemTime::now(),
        };
        let summary = report.summary_line();
        assert!(summary.contains("APPROVED"));
        assert!(summary.contains("100.0"));
    }

    #[test]
    fn test_report_summary_line_rejected() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 5,
            critical_unresolved: 2,
            overall_score: 30.0,
            recommendation: QaRecommendation::Reject("bad code".to_string()),
            generated_at: SystemTime::now(),
        };
        let summary = report.summary_line();
        assert!(summary.contains("REJECTED"));
        assert!(summary.contains("bad code"));
    }

    #[test]
    fn test_report_findings_by_file_empty() {
        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: Vec::new(),
            cross_validations: Vec::new(),
            total_findings: 0,
            critical_unresolved: 0,
            overall_score: 100.0,
            recommendation: QaRecommendation::Approve,
            generated_at: SystemTime::now(),
        };
        assert!(report.findings_by_file().is_empty());
    }

    #[test]
    fn test_report_findings_by_file_aggregation() {
        let mut a1 = QaAgent::new(QaAgentType::CompileChecker);
        a1.add_finding(make_finding(Severity::Low, "a1"));
        a1.add_finding(QaFinding::new(
            QaCategory::Security,
            Severity::High,
            PathBuf::from("src/lib.rs"),
            "issue",
            "a1",
        ));
        let mut round = QaRound::new(1);
        round.add_agent(a1);

        let report = QaReport {
            run_id: "r1".to_string(),
            rounds: vec![round],
            cross_validations: Vec::new(),
            total_findings: 2,
            critical_unresolved: 0,
            overall_score: 80.0,
            recommendation: QaRecommendation::Approve,
            generated_at: SystemTime::now(),
        };
        let by_file = report.findings_by_file();
        assert_eq!(by_file.len(), 2);
        assert_eq!(
            by_file.get(&PathBuf::from("src/main.rs")).expect("main.rs findings").len(),
            1
        );
        assert_eq!(
            by_file.get(&PathBuf::from("src/lib.rs")).expect("lib.rs findings").len(),
            1
        );
    }

    // --- Integration / edge case tests ---

    #[test]
    fn test_full_pipeline_flow() {
        let mut pipeline = QaPipeline::new();
        let round = pipeline.create_round();
        round.start();

        let mut agent = QaAgent::new(QaAgentType::CompileChecker);
        agent.start();
        agent.review_file(PathBuf::from("src/main.rs"));
        agent.add_finding(
            make_finding(Severity::Medium, "agent-compile-checker")
                .with_line(42)
                .with_suggestion("add error handling"),
        );
        agent.complete();
        round.add_agent(agent);
        round.complete();

        let report = pipeline.generate_report("full-test");
        assert_eq!(report.total_findings, 1);
        assert_eq!(report.critical_unresolved, 0);
        assert!(report.is_approved());
    }

    #[test]
    fn test_resolve_and_recheck() {
        let mut round = QaRound::new(1);
        let mut agent = QaAgent::new(QaAgentType::SecurityAuditor);
        let f = make_finding(Severity::Critical, "sa");
        let fid = f.id.clone();
        agent.add_finding(f);
        round.add_agent(agent);

        assert_eq!(round.critical_unresolved().len(), 1);
        assert!(round.resolve_finding(&fid));
        assert_eq!(round.critical_unresolved().len(), 0);
    }

    #[test]
    fn test_multiple_agents_different_types() {
        let agents = QaPipeline::new().spawn_standard_agents();
        let types: Vec<&QaAgentType> = agents.iter().map(|a| &a.agent_type).collect();
        assert!(types.contains(&&QaAgentType::CompileChecker));
        assert!(types.contains(&&QaAgentType::SecurityAuditor));
        assert!(types.contains(&&QaAgentType::IntegrationTester));
        assert!(types.contains(&&QaAgentType::DependencyAuditor));
    }

    #[test]
    fn test_round_with_all_resolved_findings() {
        let mut r = QaRound::new(1);
        let mut a = QaAgent::new(QaAgentType::CompileChecker);
        a.pass_rate = 100.0;
        let mut f1 = make_finding(Severity::Medium, "a1");
        f1.resolve();
        let mut f2 = make_finding(Severity::Low, "a1");
        f2.resolve();
        a.add_finding(f1);
        a.add_finding(f2);
        r.add_agent(a);
        r.start();
        r.complete();
        assert_eq!(r.status, QaStatus::Passed);
        assert_eq!(r.unresolved_findings().len(), 0);
    }

    #[test]
    fn test_finding_unique_ids() {
        let f1 = make_finding(Severity::Low, "a");
        let f2 = make_finding(Severity::Low, "a");
        assert_ne!(f1.id, f2.id);
    }

    #[test]
    fn test_agent_type_labels() {
        assert_eq!(QaAgentType::CompileChecker.label(), "compile-checker");
        assert_eq!(QaAgentType::TestRunner.label(), "test-runner");
        assert_eq!(QaAgentType::SecurityAuditor.label(), "security-auditor");
        assert_eq!(QaAgentType::PerformanceAnalyzer.label(), "performance-analyzer");
    }
}
