#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum SourceLanguage {
    Cobol,
    Fortran,
    Java4,
    Java5,
    Java6,
    Java7,
    CSharpLegacy,
    VB6,
    VBNet,
    Delphi,
    PowerBuilder,
    CppLegacy,
    Perl,
    Php4,
    Php5,
    Rpg,
    Abap,
    Mumps,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TargetLanguage {
    Rust,
    Go,
    Python,
    TypeScript,
    Java21,
    CSharp12,
    Kotlin,
    Swift,
    Scala,
    Ruby,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationStrategy {
    DirectTranslation,
    Rewrite,
    StranglerFig,
    BigBang,
    Incremental,
    HybridBridge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationPhase {
    Analysis,
    Planning,
    Translation,
    Validation,
    Testing,
    Deployment,
    Monitoring,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed(String),
    Paused,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentType {
    Module,
    Class,
    Function,
    Procedure,
    Copybook,
    DataStructure,
    Screen,
    Report,
    BatchJob,
    StoredProcedure,
    Trigger,
    View,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    Import,
    Include,
    Call,
    DataFlow,
    FileIO,
    DatabaseAccess,
    ExternalApi,
}

// === Core Structures ===

#[derive(Debug, Clone)]
pub struct LegacyComponent {
    pub id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub source_language: SourceLanguage,
    pub file_path: PathBuf,
    pub lines_of_code: usize,
    pub complexity_score: u32,
    pub dependencies: Vec<ComponentDependency>,
    pub data_structures: Vec<DataStructure>,
    pub business_rules: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct ComponentDependency {
    pub target_id: String,
    pub dependency_type: DependencyType,
    pub description: String,
    pub critical: bool,
}

#[derive(Debug, Clone)]
pub struct DataStructure {
    pub name: String,
    pub fields: Vec<(String, String, String)>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct MigrationPlan {
    pub id: String,
    pub title: String,
    pub source: SourceLanguage,
    pub target: TargetLanguage,
    pub strategy: MigrationStrategy,
    pub phases: Vec<MigrationPhaseDetail>,
    pub components: Vec<LegacyComponent>,
    pub dependency_graph: Vec<(String, String)>,
    pub estimated_effort_hours: u32,
    pub risk_assessment: RiskAssessment,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct MigrationPhaseDetail {
    pub phase: MigrationPhase,
    pub description: String,
    pub components: Vec<String>,
    pub estimated_hours: u32,
    pub status: MigrationStatus,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risks: Vec<RiskItem>,
    pub mitigations: Vec<String>,
    pub rollback_plan: String,
}

#[derive(Debug, Clone)]
pub struct RiskItem {
    pub description: String,
    pub risk_level: RiskLevel,
    pub probability: f64,
    pub impact: f64,
    pub mitigation: String,
}

#[derive(Debug, Clone)]
pub struct TranslationRule {
    pub id: String,
    pub source_pattern: String,
    pub target_pattern: String,
    pub source_lang: SourceLanguage,
    pub target_lang: TargetLanguage,
    pub description: String,
    pub examples: Vec<(String, String)>,
    pub confidence: f64,
    pub requires_review: bool,
}

#[derive(Debug, Clone)]
pub struct TranslationResult {
    pub component_id: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub source_lines: usize,
    pub target_lines: usize,
    pub rules_applied: Vec<String>,
    pub warnings: Vec<String>,
    pub manual_review_needed: Vec<ManualReviewItem>,
    pub confidence_score: f64,
    pub status: MigrationStatus,
}

#[derive(Debug, Clone)]
pub struct ManualReviewItem {
    pub line: usize,
    pub original: String,
    pub translated: String,
    pub reason: String,
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub struct ServiceBoundary {
    pub name: String,
    pub components: Vec<String>,
    pub api_surface: Vec<String>,
    pub data_stores: Vec<String>,
    pub estimated_size: usize,
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub plan_id: String,
    pub components_total: usize,
    pub components_migrated: usize,
    pub lines_source: usize,
    pub lines_target: usize,
    pub translation_results: Vec<TranslationResult>,
    pub service_boundaries: Vec<ServiceBoundary>,
    pub overall_confidence: f64,
    pub manual_reviews_needed: usize,
    pub duration: Duration,
    pub generated_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub max_complexity_threshold: u32,
    pub auto_service_decomposition: bool,
    pub generate_tests: bool,
    pub preserve_comments: bool,
    pub add_migration_markers: bool,
    pub parallel_translation: bool,
    pub target_test_coverage: f64,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub total_components: usize,
    pub total_lines: usize,
    pub avg_complexity: f64,
    pub high_risk_count: usize,
    pub language_breakdown: HashMap<String, usize>,
    pub component_type_breakdown: HashMap<String, usize>,
    pub estimated_effort_hours: u32,
    pub recommended_strategy: MigrationStrategy,
    pub warnings: Vec<String>,
}

pub struct MigrationEngine {
    pub plans: Vec<MigrationPlan>,
    pub translation_rules: Vec<TranslationRule>,
    pub active_plan: Option<String>,
    pub reports: Vec<MigrationReport>,
    pub config: MigrationConfig,
}

// === Helper functions ===

fn generate_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:x}-{:x}", now.as_secs(), now.subsec_nanos(), seq)
}

fn source_language_name(lang: &SourceLanguage) -> String {
    match lang {
        SourceLanguage::Cobol => "COBOL".to_string(),
        SourceLanguage::Fortran => "Fortran".to_string(),
        SourceLanguage::Java4 => "Java 4".to_string(),
        SourceLanguage::Java5 => "Java 5".to_string(),
        SourceLanguage::Java6 => "Java 6".to_string(),
        SourceLanguage::Java7 => "Java 7".to_string(),
        SourceLanguage::CSharpLegacy => "C# Legacy".to_string(),
        SourceLanguage::VB6 => "VB6".to_string(),
        SourceLanguage::VBNet => "VB.NET".to_string(),
        SourceLanguage::Delphi => "Delphi".to_string(),
        SourceLanguage::PowerBuilder => "PowerBuilder".to_string(),
        SourceLanguage::CppLegacy => "C++ Legacy".to_string(),
        SourceLanguage::Perl => "Perl".to_string(),
        SourceLanguage::Php4 => "PHP 4".to_string(),
        SourceLanguage::Php5 => "PHP 5".to_string(),
        SourceLanguage::Rpg => "RPG".to_string(),
        SourceLanguage::Abap => "ABAP".to_string(),
        SourceLanguage::Mumps => "MUMPS".to_string(),
        SourceLanguage::Custom(s) => s.clone(),
    }
}

fn component_type_name(ct: &ComponentType) -> String {
    match ct {
        ComponentType::Module => "Module".to_string(),
        ComponentType::Class => "Class".to_string(),
        ComponentType::Function => "Function".to_string(),
        ComponentType::Procedure => "Procedure".to_string(),
        ComponentType::Copybook => "Copybook".to_string(),
        ComponentType::DataStructure => "DataStructure".to_string(),
        ComponentType::Screen => "Screen".to_string(),
        ComponentType::Report => "Report".to_string(),
        ComponentType::BatchJob => "BatchJob".to_string(),
        ComponentType::StoredProcedure => "StoredProcedure".to_string(),
        ComponentType::Trigger => "Trigger".to_string(),
        ComponentType::View => "View".to_string(),
    }
}

fn target_extension(lang: &TargetLanguage) -> &str {
    match lang {
        TargetLanguage::Rust => "rs",
        TargetLanguage::Go => "go",
        TargetLanguage::Python => "py",
        TargetLanguage::TypeScript => "ts",
        TargetLanguage::Java21 => "java",
        TargetLanguage::CSharp12 => "cs",
        TargetLanguage::Kotlin => "kt",
        TargetLanguage::Swift => "swift",
        TargetLanguage::Scala => "scala",
        TargetLanguage::Ruby => "rb",
        TargetLanguage::Custom(_) => "txt",
    }
}

// === Implementations ===

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            max_complexity_threshold: 80,
            auto_service_decomposition: true,
            generate_tests: true,
            preserve_comments: true,
            add_migration_markers: true,
            parallel_translation: true,
            target_test_coverage: 80.0,
        }
    }
}

impl LegacyComponent {
    pub fn new(name: &str, comp_type: ComponentType, lang: SourceLanguage) -> Self {
        Self {
            id: generate_id(),
            name: name.to_string(),
            component_type: comp_type,
            source_language: lang,
            file_path: PathBuf::new(),
            lines_of_code: 0,
            complexity_score: 1,
            dependencies: Vec::new(),
            data_structures: Vec::new(),
            business_rules: Vec::new(),
            risk_level: RiskLevel::Low,
        }
    }

    pub fn add_dependency(&mut self, dep: ComponentDependency) {
        self.dependencies.push(dep);
    }

    pub fn add_data_structure(&mut self, ds: DataStructure) {
        self.data_structures.push(ds);
    }

    pub fn add_business_rule(&mut self, rule: &str) {
        self.business_rules.push(rule.to_string());
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(self.risk_level, RiskLevel::High | RiskLevel::Critical)
    }

    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }

    pub fn assess_risk(&mut self) {
        let critical_deps = self.dependencies.iter().filter(|d| d.critical).count();
        let has_db = self
            .dependencies
            .iter()
            .any(|d| d.dependency_type == DependencyType::DatabaseAccess);
        let has_external = self
            .dependencies
            .iter()
            .any(|d| d.dependency_type == DependencyType::ExternalApi);

        if self.complexity_score >= 80 || critical_deps >= 5 {
            self.risk_level = RiskLevel::Critical;
        } else if self.complexity_score >= 50
            || critical_deps >= 3
            || (has_db && has_external)
            || self.lines_of_code > 5000
        {
            self.risk_level = RiskLevel::High;
        } else if self.complexity_score >= 25
            || critical_deps >= 1
            || has_db
            || self.lines_of_code > 1000
        {
            self.risk_level = RiskLevel::Medium;
        } else {
            self.risk_level = RiskLevel::Low;
        }
    }
}

impl MigrationPlan {
    pub fn new(
        title: &str,
        source: SourceLanguage,
        target: TargetLanguage,
        strategy: MigrationStrategy,
    ) -> Self {
        Self {
            id: generate_id(),
            title: title.to_string(),
            source,
            target,
            strategy,
            phases: Vec::new(),
            components: Vec::new(),
            dependency_graph: Vec::new(),
            estimated_effort_hours: 0,
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risks: Vec::new(),
                mitigations: Vec::new(),
                rollback_plan: String::new(),
            },
            created_at: SystemTime::now(),
        }
    }

    pub fn add_component(&mut self, component: LegacyComponent) {
        // Update estimated effort: ~1 hour per 100 lines + complexity factor
        let effort = (component.lines_of_code as u32 / 100).max(1)
            + component.complexity_score / 10;
        self.estimated_effort_hours += effort;
        self.components.push(component);
    }

    pub fn add_phase(&mut self, phase: MigrationPhaseDetail) {
        self.phases.push(phase);
    }

    pub fn build_dependency_graph(&mut self) {
        self.dependency_graph.clear();
        for component in &self.components {
            for dep in &component.dependencies {
                // Only add edge if target component exists in the plan
                if self.components.iter().any(|c| c.id == dep.target_id) {
                    self.dependency_graph
                        .push((component.id.clone(), dep.target_id.clone()));
                }
            }
        }
    }

    pub fn topological_order(&self) -> Vec<String> {
        let ids: Vec<&str> = self.components.iter().map(|c| c.id.as_str()).collect();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in &ids {
            in_degree.insert(id, 0);
            adj.insert(id, Vec::new());
        }

        // edges are (from, to) meaning "from depends on to", so to must come first
        for (from, to) in &self.dependency_graph {
            if let Some(list) = adj.get_mut(to.as_str()) {
                list.push(from.as_str());
            }
            if let Some(deg) = in_degree.get_mut(from.as_str()) {
                *deg += 1;
            }
        }

        let mut queue: Vec<&str> = ids
            .iter()
            .filter(|id| in_degree.get(*id).copied().unwrap_or(0) == 0)
            .copied()
            .collect();
        queue.sort(); // deterministic ordering

        let mut result = Vec::new();
        while let Some(node) = queue.pop() {
            result.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(neighbor);
                            queue.sort();
                        }
                    }
                }
            }
        }

        result
    }

    pub fn total_lines(&self) -> usize {
        self.components.iter().map(|c| c.lines_of_code).sum()
    }

    pub fn total_complexity(&self) -> u32 {
        self.components.iter().map(|c| c.complexity_score).sum()
    }

    pub fn highest_risk_components(&self) -> Vec<&LegacyComponent> {
        let mut critical: Vec<&LegacyComponent> = self
            .components
            .iter()
            .filter(|c| c.risk_level == RiskLevel::Critical)
            .collect();
        if critical.is_empty() {
            critical = self
                .components
                .iter()
                .filter(|c| c.risk_level == RiskLevel::High)
                .collect();
        }
        critical.sort_by(|a, b| b.complexity_score.cmp(&a.complexity_score));
        critical
    }

    pub fn phase_status(&self, phase: &MigrationPhase) -> Option<&MigrationStatus> {
        self.phases
            .iter()
            .find(|p| p.phase == *phase)
            .map(|p| &p.status)
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.phases.is_empty() {
            return 0.0;
        }
        let completed = self
            .phases
            .iter()
            .filter(|p| matches!(p.status, MigrationStatus::Completed))
            .count();
        (completed as f64 / self.phases.len() as f64) * 100.0
    }

    pub fn estimated_remaining_hours(&self) -> u32 {
        self.phases
            .iter()
            .filter(|p| !matches!(p.status, MigrationStatus::Completed))
            .map(|p| p.estimated_hours)
            .sum()
    }
}

impl TranslationRule {
    pub fn new(
        source_lang: SourceLanguage,
        target_lang: TargetLanguage,
        source_pat: &str,
        target_pat: &str,
    ) -> Self {
        Self {
            id: generate_id(),
            source_pattern: source_pat.to_string(),
            target_pattern: target_pat.to_string(),
            source_lang,
            target_lang,
            description: format!("Translate '{}' to '{}'", source_pat, target_pat),
            examples: Vec::new(),
            confidence: 0.8,
            requires_review: false,
        }
    }

    pub fn with_example(mut self, before: &str, after: &str) -> Self {
        self.examples.push((before.to_string(), after.to_string()));
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn needs_review(mut self) -> Self {
        self.requires_review = true;
        self
    }

    pub fn apply(&self, input: &str) -> Option<String> {
        if input.contains(&self.source_pattern) {
            Some(input.replace(&self.source_pattern, &self.target_pattern))
        } else {
            None
        }
    }
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            total_components: 0,
            total_lines: 0,
            avg_complexity: 0.0,
            high_risk_count: 0,
            language_breakdown: HashMap::new(),
            component_type_breakdown: HashMap::new(),
            estimated_effort_hours: 0,
            recommended_strategy: MigrationStrategy::Incremental,
            warnings: Vec::new(),
        }
    }

    pub fn is_suitable_for_big_bang(&self) -> bool {
        self.total_components <= 20 && self.avg_complexity < 30.0 && self.high_risk_count == 0
    }

    pub fn is_suitable_for_strangler(&self) -> bool {
        self.total_components > 10 || self.avg_complexity >= 30.0 || self.high_risk_count > 0
    }

    pub fn summary(&self) -> String {
        format!(
            "Analysis: {} components, {} lines, avg complexity {:.1}, {} high-risk. \
             Estimated effort: {} hours. Recommended strategy: {:?}.",
            self.total_components,
            self.total_lines,
            self.avg_complexity,
            self.high_risk_count,
            self.estimated_effort_hours,
            self.recommended_strategy,
        )
    }
}

impl MigrationEngine {
    pub fn new() -> Self {
        Self {
            plans: Vec::new(),
            translation_rules: Vec::new(),
            active_plan: None,
            reports: Vec::new(),
            config: MigrationConfig::default(),
        }
    }

    pub fn with_config(config: MigrationConfig) -> Self {
        Self {
            plans: Vec::new(),
            translation_rules: Vec::new(),
            active_plan: None,
            reports: Vec::new(),
            config,
        }
    }

    pub fn create_plan(
        &mut self,
        title: &str,
        source: SourceLanguage,
        target: TargetLanguage,
        strategy: MigrationStrategy,
    ) -> &mut MigrationPlan {
        let plan = MigrationPlan::new(title, source, target, strategy);
        let id = plan.id.clone();
        self.plans.push(plan);
        self.active_plan = Some(id);
        self.plans.last_mut().expect("just pushed a plan")
    }

    pub fn add_translation_rule(&mut self, rule: TranslationRule) {
        self.translation_rules.push(rule);
    }

    pub fn load_default_rules(&mut self, source: &SourceLanguage, target: &TargetLanguage) {
        let rules = Self::default_rules_for_pair(source, target);
        self.translation_rules.extend(rules);
    }

    pub fn analyze_codebase(&self, components: &[LegacyComponent]) -> AnalysisResult {
        let mut result = AnalysisResult::new();
        result.total_components = components.len();
        result.total_lines = components.iter().map(|c| c.lines_of_code).sum();

        if !components.is_empty() {
            let total_complexity: u32 = components.iter().map(|c| c.complexity_score).sum();
            result.avg_complexity = total_complexity as f64 / components.len() as f64;
        }

        result.high_risk_count = components
            .iter()
            .filter(|c| matches!(c.risk_level, RiskLevel::High | RiskLevel::Critical))
            .count();

        for comp in components {
            let lang_name = source_language_name(&comp.source_language);
            *result.language_breakdown.entry(lang_name).or_insert(0) += 1;

            let type_name = component_type_name(&comp.component_type);
            *result
                .component_type_breakdown
                .entry(type_name)
                .or_insert(0) += 1;
        }

        // Estimate effort: base of 2 hours per component + 1 hour per 200 lines + complexity factor
        result.estimated_effort_hours = components
            .iter()
            .map(|c| {
                2 + (c.lines_of_code as u32 / 200).max(1) + c.complexity_score / 15
            })
            .sum();

        result.recommended_strategy = self.suggest_strategy(&result);

        // Generate warnings
        if result.avg_complexity > 60.0 {
            result
                .warnings
                .push("Very high average complexity — consider decomposition first.".to_string());
        }
        if result.high_risk_count > result.total_components / 2 {
            result
                .warnings
                .push("Majority of components are high risk.".to_string());
        }
        if result.total_lines > 500_000 {
            result
                .warnings
                .push("Very large codebase — phased approach strongly recommended.".to_string());
        }

        result
    }

    pub fn suggest_strategy(&self, analysis: &AnalysisResult) -> MigrationStrategy {
        if analysis.is_suitable_for_big_bang() {
            MigrationStrategy::BigBang
        } else if analysis.high_risk_count == 0 && analysis.total_components <= 50 {
            MigrationStrategy::DirectTranslation
        } else if analysis.is_suitable_for_strangler() {
            MigrationStrategy::StranglerFig
        } else {
            MigrationStrategy::Incremental
        }
    }

    pub fn identify_service_boundaries(
        &self,
        components: &[LegacyComponent],
    ) -> Vec<ServiceBoundary> {
        // Group components by shared dependencies and data access patterns
        let mut boundaries: Vec<ServiceBoundary> = Vec::new();

        // Group by dependency clusters: components that share database/external deps
        let mut db_groups: HashMap<String, Vec<&LegacyComponent>> = HashMap::new();
        let mut ungrouped: Vec<&LegacyComponent> = Vec::new();

        for comp in components {
            let db_deps: Vec<String> = comp
                .dependencies
                .iter()
                .filter(|d| {
                    d.dependency_type == DependencyType::DatabaseAccess
                        || d.dependency_type == DependencyType::FileIO
                })
                .map(|d| d.target_id.clone())
                .collect();

            if db_deps.is_empty() {
                ungrouped.push(comp);
            } else {
                let key = db_deps.join(",");
                db_groups.entry(key).or_default().push(comp);
            }
        }

        let mut idx = 0;
        for (data_key, group) in &db_groups {
            let total_lines: usize = group.iter().map(|c| c.lines_of_code).sum();
            let comp_ids: Vec<String> = group.iter().map(|c| c.id.clone()).collect();
            let api_surface: Vec<String> = group
                .iter()
                .flat_map(|c| {
                    c.dependencies
                        .iter()
                        .filter(|d| d.dependency_type == DependencyType::ExternalApi)
                        .map(|d| d.description.clone())
                })
                .collect();

            let data_stores: Vec<String> = data_key.split(',').map(|s| s.to_string()).collect();

            boundaries.push(ServiceBoundary {
                name: format!("service-{}", idx),
                components: comp_ids,
                api_surface,
                data_stores,
                estimated_size: total_lines,
            });
            idx += 1;
        }

        // Group remaining ungrouped components into a single "core" boundary
        if !ungrouped.is_empty() {
            let total_lines: usize = ungrouped.iter().map(|c| c.lines_of_code).sum();
            boundaries.push(ServiceBoundary {
                name: format!("service-core-{}", idx),
                components: ungrouped.iter().map(|c| c.id.clone()).collect(),
                api_surface: Vec::new(),
                data_stores: Vec::new(),
                estimated_size: total_lines,
            });
        }

        boundaries
    }

    pub fn translate_component(
        &self,
        component: &LegacyComponent,
        target: &TargetLanguage,
    ) -> TranslationResult {
        let ext = target_extension(target);
        let target_path = component
            .file_path
            .with_extension(ext);

        let matching_rules: Vec<&TranslationRule> = self
            .translation_rules
            .iter()
            .filter(|r| r.source_lang == component.source_language && r.target_lang == *target)
            .collect();

        let rules_applied: Vec<String> = matching_rules.iter().map(|r| r.id.clone()).collect();

        let confidence = if matching_rules.is_empty() {
            0.3
        } else {
            let sum: f64 = matching_rules.iter().map(|r| r.confidence).sum();
            (sum / matching_rules.len() as f64).min(1.0)
        };

        let mut warnings = Vec::new();
        let mut manual_reviews = Vec::new();

        if component.complexity_score > self.config.max_complexity_threshold {
            warnings.push(format!(
                "Complexity {} exceeds threshold {}",
                component.complexity_score, self.config.max_complexity_threshold
            ));
        }

        for rule in &matching_rules {
            if rule.requires_review {
                manual_reviews.push(ManualReviewItem {
                    line: 0,
                    original: rule.source_pattern.clone(),
                    translated: rule.target_pattern.clone(),
                    reason: "Translation rule flagged for review".to_string(),
                    suggestion: format!("Verify '{}' translation", rule.description),
                });
            }
        }

        if component.is_high_risk() {
            warnings.push("High-risk component — thorough testing required.".to_string());
        }

        // Estimate target lines (modern languages are typically more concise for COBOL/Fortran,
        // but could be similar or larger for already-modern legacy languages)
        let target_lines = match (&component.source_language, target) {
            (SourceLanguage::Cobol, _) => component.lines_of_code * 60 / 100,
            (SourceLanguage::Fortran, _) => component.lines_of_code * 70 / 100,
            (SourceLanguage::VB6, _) => component.lines_of_code * 75 / 100,
            _ => component.lines_of_code * 85 / 100,
        };

        let status = if manual_reviews.is_empty() && warnings.is_empty() {
            MigrationStatus::Completed
        } else {
            MigrationStatus::InProgress
        };

        TranslationResult {
            component_id: component.id.clone(),
            source_path: component.file_path.clone(),
            target_path,
            source_lines: component.lines_of_code,
            target_lines,
            rules_applied,
            warnings,
            manual_review_needed: manual_reviews,
            confidence_score: confidence,
            status,
        }
    }

    pub fn generate_report(&self, plan_id: &str) -> Option<MigrationReport> {
        let plan = self.plans.iter().find(|p| p.id == plan_id)?;

        let start = SystemTime::now();
        let translation_results: Vec<TranslationResult> = plan
            .components
            .iter()
            .map(|c| self.translate_component(c, &plan.target))
            .collect();

        let lines_source: usize = translation_results.iter().map(|r| r.source_lines).sum();
        let lines_target: usize = translation_results.iter().map(|r| r.target_lines).sum();
        let migrated = translation_results
            .iter()
            .filter(|r| matches!(r.status, MigrationStatus::Completed))
            .count();
        let reviews_needed: usize = translation_results
            .iter()
            .map(|r| r.manual_review_needed.len())
            .sum();
        let overall_confidence = if translation_results.is_empty() {
            0.0
        } else {
            let sum: f64 = translation_results.iter().map(|r| r.confidence_score).sum();
            sum / translation_results.len() as f64
        };

        let service_boundaries = self.identify_service_boundaries(&plan.components);

        let duration = start.elapsed().unwrap_or(Duration::from_secs(0));

        Some(MigrationReport {
            plan_id: plan_id.to_string(),
            components_total: plan.components.len(),
            components_migrated: migrated,
            lines_source,
            lines_target,
            translation_results,
            service_boundaries,
            overall_confidence,
            manual_reviews_needed: reviews_needed,
            duration,
            generated_at: SystemTime::now(),
        })
    }

    pub fn default_rules_for_pair(
        source: &SourceLanguage,
        target: &TargetLanguage,
    ) -> Vec<TranslationRule> {
        let mut rules = Vec::new();

        match (source, target) {
            (SourceLanguage::Cobol, TargetLanguage::Rust) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "PERFORM",
                        "fn",
                    )
                    .with_example("PERFORM CALCULATE-TOTAL", "fn calculate_total()")
                    .with_confidence(0.9),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "MOVE",
                        "let",
                    )
                    .with_example("MOVE A TO B", "let b = a;")
                    .with_confidence(0.85),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "PIC 9",
                        "i64",
                    )
                    .with_confidence(0.8),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "PIC X",
                        "String",
                    )
                    .with_confidence(0.8),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "IF",
                        "if",
                    )
                    .with_confidence(0.95),
                );
            }
            (SourceLanguage::Cobol, TargetLanguage::Java21) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "PERFORM",
                        "void",
                    )
                    .with_example("PERFORM CALC", "void calc()")
                    .with_confidence(0.85),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "PIC 9",
                        "long",
                    )
                    .with_confidence(0.8),
                );
            }
            (SourceLanguage::Java4 | SourceLanguage::Java5 | SourceLanguage::Java6 | SourceLanguage::Java7, TargetLanguage::Kotlin) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "public class",
                        "class",
                    )
                    .with_confidence(0.95),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "ArrayList",
                        "mutableListOf",
                    )
                    .with_confidence(0.9),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "HashMap",
                        "mutableMapOf",
                    )
                    .with_confidence(0.9),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "null",
                        "null",
                    )
                    .with_confidence(0.7)
                    .needs_review(),
                );
            }
            (SourceLanguage::VB6, TargetLanguage::CSharp12) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "Dim",
                        "var",
                    )
                    .with_confidence(0.9),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "Sub",
                        "void",
                    )
                    .with_confidence(0.85),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "MsgBox",
                        "MessageBox.Show",
                    )
                    .with_confidence(0.9),
                );
            }
            (SourceLanguage::Fortran, TargetLanguage::Python) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "SUBROUTINE",
                        "def",
                    )
                    .with_confidence(0.9),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "DO",
                        "for",
                    )
                    .with_confidence(0.85),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "REAL",
                        "float",
                    )
                    .with_confidence(0.9),
                );
            }
            (SourceLanguage::CSharpLegacy, TargetLanguage::CSharp12) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "delegate",
                        "Func",
                    )
                    .with_confidence(0.8)
                    .needs_review(),
                );
            }
            (SourceLanguage::Perl, TargetLanguage::Python) => {
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "my $",
                        "",
                    )
                    .with_confidence(0.7)
                    .needs_review(),
                );
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "sub ",
                        "def ",
                    )
                    .with_confidence(0.85),
                );
            }
            _ => {
                // Generic fallback rule
                rules.push(
                    TranslationRule::new(
                        source.clone(),
                        target.clone(),
                        "//",
                        "//",
                    )
                    .with_confidence(0.5)
                    .needs_review(),
                );
            }
        }

        rules
    }

    pub fn get_plan(&self, plan_id: &str) -> Option<&MigrationPlan> {
        self.plans.iter().find(|p| p.id == plan_id)
    }

    pub fn list_plans(&self) -> Vec<(&str, &str)> {
        self.plans
            .iter()
            .map(|p| (p.id.as_str(), p.title.as_str()))
            .collect()
    }

    pub fn supported_pairs() -> Vec<(SourceLanguage, TargetLanguage)> {
        vec![
            (SourceLanguage::Cobol, TargetLanguage::Rust),
            (SourceLanguage::Cobol, TargetLanguage::Java21),
            (SourceLanguage::Cobol, TargetLanguage::CSharp12),
            (SourceLanguage::Cobol, TargetLanguage::Python),
            (SourceLanguage::Cobol, TargetLanguage::TypeScript),
            (SourceLanguage::Fortran, TargetLanguage::Rust),
            (SourceLanguage::Fortran, TargetLanguage::Python),
            (SourceLanguage::Fortran, TargetLanguage::Go),
            (SourceLanguage::Java4, TargetLanguage::Java21),
            (SourceLanguage::Java4, TargetLanguage::Kotlin),
            (SourceLanguage::Java5, TargetLanguage::Java21),
            (SourceLanguage::Java5, TargetLanguage::Kotlin),
            (SourceLanguage::Java6, TargetLanguage::Java21),
            (SourceLanguage::Java6, TargetLanguage::Kotlin),
            (SourceLanguage::Java7, TargetLanguage::Java21),
            (SourceLanguage::Java7, TargetLanguage::Kotlin),
            (SourceLanguage::CSharpLegacy, TargetLanguage::CSharp12),
            (SourceLanguage::VB6, TargetLanguage::CSharp12),
            (SourceLanguage::VBNet, TargetLanguage::CSharp12),
            (SourceLanguage::Delphi, TargetLanguage::CSharp12),
            (SourceLanguage::Delphi, TargetLanguage::Rust),
            (SourceLanguage::PowerBuilder, TargetLanguage::CSharp12),
            (SourceLanguage::CppLegacy, TargetLanguage::Rust),
            (SourceLanguage::CppLegacy, TargetLanguage::Go),
            (SourceLanguage::Perl, TargetLanguage::Python),
            (SourceLanguage::Perl, TargetLanguage::Ruby),
            (SourceLanguage::Php4, TargetLanguage::TypeScript),
            (SourceLanguage::Php5, TargetLanguage::TypeScript),
            (SourceLanguage::Rpg, TargetLanguage::Java21),
            (SourceLanguage::Abap, TargetLanguage::Java21),
            (SourceLanguage::Mumps, TargetLanguage::Python),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a basic component
    fn make_component(name: &str, lang: SourceLanguage, loc: usize, complexity: u32) -> LegacyComponent {
        let mut c = LegacyComponent::new(name, ComponentType::Module, lang);
        c.lines_of_code = loc;
        c.complexity_score = complexity;
        c
    }

    fn make_dep(target_id: &str, dep_type: DependencyType, critical: bool) -> ComponentDependency {
        ComponentDependency {
            target_id: target_id.to_string(),
            dependency_type: dep_type,
            description: format!("dep on {}", target_id),
            critical,
        }
    }

    // --- Component creation tests ---

    #[test]
    fn test_component_new() {
        let c = LegacyComponent::new("main-module", ComponentType::Module, SourceLanguage::Cobol);
        assert_eq!(c.name, "main-module");
        assert_eq!(c.component_type, ComponentType::Module);
        assert_eq!(c.source_language, SourceLanguage::Cobol);
        assert_eq!(c.risk_level, RiskLevel::Low);
        assert!(c.dependencies.is_empty());
    }

    #[test]
    fn test_component_new_class() {
        let c = LegacyComponent::new("UserManager", ComponentType::Class, SourceLanguage::Java4);
        assert_eq!(c.component_type, ComponentType::Class);
        assert_eq!(c.source_language, SourceLanguage::Java4);
    }

    #[test]
    fn test_component_add_dependency() {
        let mut c = LegacyComponent::new("mod-a", ComponentType::Module, SourceLanguage::Cobol);
        c.add_dependency(make_dep("mod-b", DependencyType::Call, false));
        assert_eq!(c.dependency_count(), 1);
    }

    #[test]
    fn test_component_add_multiple_deps() {
        let mut c = LegacyComponent::new("mod-a", ComponentType::Module, SourceLanguage::Cobol);
        c.add_dependency(make_dep("b", DependencyType::Call, false));
        c.add_dependency(make_dep("c", DependencyType::Import, true));
        c.add_dependency(make_dep("d", DependencyType::DataFlow, false));
        assert_eq!(c.dependency_count(), 3);
    }

    #[test]
    fn test_component_add_data_structure() {
        let mut c = LegacyComponent::new("mod", ComponentType::Module, SourceLanguage::Cobol);
        c.add_data_structure(DataStructure {
            name: "CUSTOMER-REC".to_string(),
            fields: vec![
                ("CUST-ID".to_string(), "PIC 9(5)".to_string(), "i64".to_string()),
                ("CUST-NAME".to_string(), "PIC X(30)".to_string(), "String".to_string()),
            ],
            source: "COPYBOOK".to_string(),
        });
        assert_eq!(c.data_structures.len(), 1);
        assert_eq!(c.data_structures[0].fields.len(), 2);
    }

    #[test]
    fn test_component_add_business_rule() {
        let mut c = LegacyComponent::new("mod", ComponentType::Module, SourceLanguage::Cobol);
        c.add_business_rule("Tax rate is 15% for domestic transactions");
        c.add_business_rule("Discount applies for orders over $1000");
        assert_eq!(c.business_rules.len(), 2);
    }

    // --- Risk assessment tests ---

    #[test]
    fn test_risk_low_simple_component() {
        let mut c = make_component("simple", SourceLanguage::Cobol, 100, 5);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Low);
        assert!(!c.is_high_risk());
    }

    #[test]
    fn test_risk_medium_from_complexity() {
        let mut c = make_component("medium", SourceLanguage::Cobol, 100, 30);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_risk_medium_from_db_access() {
        let mut c = make_component("db-user", SourceLanguage::Cobol, 100, 10);
        c.add_dependency(make_dep("db", DependencyType::DatabaseAccess, false));
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_risk_medium_from_lines() {
        let mut c = make_component("big", SourceLanguage::Cobol, 2000, 10);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_risk_high_from_complexity() {
        let mut c = make_component("complex", SourceLanguage::Cobol, 100, 55);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::High);
        assert!(c.is_high_risk());
    }

    #[test]
    fn test_risk_high_from_lines() {
        let mut c = make_component("huge", SourceLanguage::Cobol, 6000, 10);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_risk_high_from_db_and_external() {
        let mut c = make_component("integrated", SourceLanguage::Cobol, 100, 10);
        c.add_dependency(make_dep("db", DependencyType::DatabaseAccess, false));
        c.add_dependency(make_dep("api", DependencyType::ExternalApi, false));
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_risk_critical_from_complexity() {
        let mut c = make_component("nightmare", SourceLanguage::Cobol, 100, 85);
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Critical);
        assert!(c.is_high_risk());
    }

    #[test]
    fn test_risk_critical_from_many_critical_deps() {
        let mut c = make_component("hub", SourceLanguage::Cobol, 100, 10);
        for i in 0..5 {
            c.add_dependency(make_dep(&format!("dep-{}", i), DependencyType::Call, true));
        }
        c.assess_risk();
        assert_eq!(c.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_is_high_risk_false_for_low() {
        let c = make_component("safe", SourceLanguage::Cobol, 50, 5);
        assert!(!c.is_high_risk());
    }

    // --- Plan tests ---

    #[test]
    fn test_plan_new() {
        let p = MigrationPlan::new("COBOL to Rust", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::StranglerFig);
        assert_eq!(p.title, "COBOL to Rust");
        assert_eq!(p.source, SourceLanguage::Cobol);
        assert_eq!(p.target, TargetLanguage::Rust);
        assert_eq!(p.strategy, MigrationStrategy::StranglerFig);
        assert!(p.components.is_empty());
    }

    #[test]
    fn test_plan_add_component() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_component(make_component("a", SourceLanguage::Cobol, 500, 20));
        assert_eq!(p.components.len(), 1);
        assert!(p.estimated_effort_hours > 0);
    }

    #[test]
    fn test_plan_total_lines() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_component(make_component("a", SourceLanguage::Cobol, 500, 10));
        p.add_component(make_component("b", SourceLanguage::Cobol, 300, 10));
        assert_eq!(p.total_lines(), 800);
    }

    #[test]
    fn test_plan_total_complexity() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_component(make_component("a", SourceLanguage::Cobol, 100, 30));
        p.add_component(make_component("b", SourceLanguage::Cobol, 100, 45));
        assert_eq!(p.total_complexity(), 75);
    }

    #[test]
    fn test_plan_highest_risk_critical() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut c1 = make_component("safe", SourceLanguage::Cobol, 100, 5);
        c1.risk_level = RiskLevel::Low;
        let mut c2 = make_component("risky", SourceLanguage::Cobol, 100, 90);
        c2.risk_level = RiskLevel::Critical;
        p.add_component(c1);
        p.add_component(c2);
        let highest = p.highest_risk_components();
        assert_eq!(highest.len(), 1);
        assert_eq!(highest[0].name, "risky");
    }

    #[test]
    fn test_plan_highest_risk_falls_back_to_high() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut c1 = make_component("low", SourceLanguage::Cobol, 100, 5);
        c1.risk_level = RiskLevel::Low;
        let mut c2 = make_component("high", SourceLanguage::Cobol, 100, 60);
        c2.risk_level = RiskLevel::High;
        p.add_component(c1);
        p.add_component(c2);
        let highest = p.highest_risk_components();
        assert_eq!(highest.len(), 1);
        assert_eq!(highest[0].name, "high");
    }

    #[test]
    fn test_plan_progress_no_phases() {
        let p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        assert_eq!(p.progress_percentage(), 0.0);
    }

    #[test]
    fn test_plan_progress_half() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Analysis,
            description: "Analyze".to_string(),
            components: vec![],
            estimated_hours: 10,
            status: MigrationStatus::Completed,
            prerequisites: vec![],
        });
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Translation,
            description: "Translate".to_string(),
            components: vec![],
            estimated_hours: 20,
            status: MigrationStatus::InProgress,
            prerequisites: vec!["Analysis".to_string()],
        });
        assert_eq!(p.progress_percentage(), 50.0);
    }

    #[test]
    fn test_plan_progress_all_done() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Analysis,
            description: "A".to_string(),
            components: vec![],
            estimated_hours: 5,
            status: MigrationStatus::Completed,
            prerequisites: vec![],
        });
        assert_eq!(p.progress_percentage(), 100.0);
    }

    #[test]
    fn test_plan_estimated_remaining() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Analysis,
            description: "A".to_string(),
            components: vec![],
            estimated_hours: 10,
            status: MigrationStatus::Completed,
            prerequisites: vec![],
        });
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Translation,
            description: "T".to_string(),
            components: vec![],
            estimated_hours: 40,
            status: MigrationStatus::InProgress,
            prerequisites: vec![],
        });
        assert_eq!(p.estimated_remaining_hours(), 40);
    }

    #[test]
    fn test_plan_phase_status() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_phase(MigrationPhaseDetail {
            phase: MigrationPhase::Testing,
            description: "Test".to_string(),
            components: vec![],
            estimated_hours: 15,
            status: MigrationStatus::Paused,
            prerequisites: vec![],
        });
        assert_eq!(p.phase_status(&MigrationPhase::Testing), Some(&MigrationStatus::Paused));
        assert_eq!(p.phase_status(&MigrationPhase::Deployment), None);
    }

    // --- Dependency graph and topological order ---

    #[test]
    fn test_build_dependency_graph() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut a = make_component("a", SourceLanguage::Cobol, 100, 10);
        let b = make_component("b", SourceLanguage::Cobol, 100, 10);
        a.add_dependency(make_dep(&b.id, DependencyType::Call, false));
        let b_id = b.id.clone();
        let a_id = a.id.clone();
        p.add_component(a);
        p.add_component(b);
        p.build_dependency_graph();
        assert_eq!(p.dependency_graph.len(), 1);
        assert_eq!(p.dependency_graph[0].0, a_id);
        assert_eq!(p.dependency_graph[0].1, b_id);
    }

    #[test]
    fn test_topological_order_simple() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut a = make_component("a", SourceLanguage::Cobol, 100, 10);
        let b = make_component("b", SourceLanguage::Cobol, 100, 10);
        // a depends on b, so b should come first
        a.add_dependency(make_dep(&b.id, DependencyType::Call, false));
        let b_id = b.id.clone();
        let a_id = a.id.clone();
        p.add_component(a);
        p.add_component(b);
        p.build_dependency_graph();
        let order = p.topological_order();
        assert_eq!(order.len(), 2);
        let b_pos = order.iter().position(|x| x == &b_id).expect("b in order");
        let a_pos = order.iter().position(|x| x == &a_id).expect("a in order");
        assert!(b_pos < a_pos, "b must come before a");
    }

    #[test]
    fn test_topological_order_no_deps() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        p.add_component(make_component("x", SourceLanguage::Cobol, 100, 10));
        p.add_component(make_component("y", SourceLanguage::Cobol, 100, 10));
        p.build_dependency_graph();
        let order = p.topological_order();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_dependency_graph_ignores_external() {
        let mut p = MigrationPlan::new("test", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut a = make_component("a", SourceLanguage::Cobol, 100, 10);
        a.add_dependency(make_dep("nonexistent", DependencyType::Call, false));
        p.add_component(a);
        p.build_dependency_graph();
        assert!(p.dependency_graph.is_empty());
    }

    // --- Translation rule tests ---

    #[test]
    fn test_rule_new() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn");
        assert_eq!(r.source_pattern, "PERFORM");
        assert_eq!(r.target_pattern, "fn");
        assert_eq!(r.confidence, 0.8);
        assert!(!r.requires_review);
    }

    #[test]
    fn test_rule_with_example() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn")
            .with_example("PERFORM CALC", "fn calc()");
        assert_eq!(r.examples.len(), 1);
        assert_eq!(r.examples[0].0, "PERFORM CALC");
    }

    #[test]
    fn test_rule_with_confidence() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn")
            .with_confidence(0.95);
        assert_eq!(r.confidence, 0.95);
    }

    #[test]
    fn test_rule_confidence_clamped() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn")
            .with_confidence(1.5);
        assert_eq!(r.confidence, 1.0);
    }

    #[test]
    fn test_rule_confidence_clamped_negative() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn")
            .with_confidence(-0.5);
        assert_eq!(r.confidence, 0.0);
    }

    #[test]
    fn test_rule_needs_review() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn")
            .needs_review();
        assert!(r.requires_review);
    }

    #[test]
    fn test_rule_apply_match() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn");
        let result = r.apply("PERFORM CALCULATE-TOTAL");
        assert_eq!(result, Some("fn CALCULATE-TOTAL".to_string()));
    }

    #[test]
    fn test_rule_apply_no_match() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn");
        let result = r.apply("MOVE A TO B");
        assert!(result.is_none());
    }

    #[test]
    fn test_rule_apply_multiple_occurrences() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "MOVE", "let");
        let result = r.apply("MOVE A MOVE B");
        assert_eq!(result, Some("let A let B".to_string()));
    }

    #[test]
    fn test_rule_apply_empty_input() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn");
        assert!(r.apply("").is_none());
    }

    #[test]
    fn test_rule_chain_builders() {
        let r = TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "IF", "if")
            .with_confidence(0.95)
            .with_example("IF X > 0", "if x > 0")
            .needs_review();
        assert_eq!(r.confidence, 0.95);
        assert!(r.requires_review);
        assert_eq!(r.examples.len(), 1);
    }

    // --- Analysis result tests ---

    #[test]
    fn test_analysis_result_new() {
        let a = AnalysisResult::new();
        assert_eq!(a.total_components, 0);
        assert_eq!(a.total_lines, 0);
        assert_eq!(a.avg_complexity, 0.0);
    }

    #[test]
    fn test_analysis_suitable_for_big_bang() {
        let mut a = AnalysisResult::new();
        a.total_components = 10;
        a.avg_complexity = 15.0;
        a.high_risk_count = 0;
        assert!(a.is_suitable_for_big_bang());
    }

    #[test]
    fn test_analysis_not_suitable_for_big_bang_too_many() {
        let mut a = AnalysisResult::new();
        a.total_components = 50;
        a.avg_complexity = 15.0;
        a.high_risk_count = 0;
        assert!(!a.is_suitable_for_big_bang());
    }

    #[test]
    fn test_analysis_not_suitable_for_big_bang_high_risk() {
        let mut a = AnalysisResult::new();
        a.total_components = 5;
        a.avg_complexity = 10.0;
        a.high_risk_count = 2;
        assert!(!a.is_suitable_for_big_bang());
    }

    #[test]
    fn test_analysis_suitable_for_strangler() {
        let mut a = AnalysisResult::new();
        a.total_components = 50;
        a.avg_complexity = 40.0;
        a.high_risk_count = 5;
        assert!(a.is_suitable_for_strangler());
    }

    #[test]
    fn test_analysis_suitable_for_strangler_many_components() {
        let mut a = AnalysisResult::new();
        a.total_components = 15;
        a.avg_complexity = 10.0;
        a.high_risk_count = 0;
        assert!(a.is_suitable_for_strangler());
    }

    #[test]
    fn test_analysis_summary() {
        let mut a = AnalysisResult::new();
        a.total_components = 10;
        a.total_lines = 5000;
        a.avg_complexity = 25.0;
        a.high_risk_count = 2;
        a.estimated_effort_hours = 100;
        let s = a.summary();
        assert!(s.contains("10 components"));
        assert!(s.contains("5000 lines"));
    }

    // --- Engine tests ---

    #[test]
    fn test_engine_new() {
        let e = MigrationEngine::new();
        assert!(e.plans.is_empty());
        assert!(e.translation_rules.is_empty());
        assert!(e.active_plan.is_none());
        assert_eq!(e.config.max_complexity_threshold, 80);
    }

    #[test]
    fn test_engine_with_config() {
        let cfg = MigrationConfig {
            max_complexity_threshold: 50,
            auto_service_decomposition: false,
            generate_tests: false,
            preserve_comments: false,
            add_migration_markers: false,
            parallel_translation: false,
            target_test_coverage: 60.0,
        };
        let e = MigrationEngine::with_config(cfg);
        assert_eq!(e.config.max_complexity_threshold, 50);
        assert!(!e.config.generate_tests);
    }

    #[test]
    fn test_engine_create_plan() {
        let mut e = MigrationEngine::new();
        let plan = e.create_plan("My Plan", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::StranglerFig);
        assert_eq!(plan.title, "My Plan");
        assert_eq!(e.plans.len(), 1);
        assert!(e.active_plan.is_some());
    }

    #[test]
    fn test_engine_create_multiple_plans() {
        let mut e = MigrationEngine::new();
        e.create_plan("Plan 1", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        e.create_plan("Plan 2", SourceLanguage::Java4, TargetLanguage::Kotlin, MigrationStrategy::Incremental);
        assert_eq!(e.plans.len(), 2);
    }

    #[test]
    fn test_engine_list_plans() {
        let mut e = MigrationEngine::new();
        e.create_plan("Alpha", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        e.create_plan("Beta", SourceLanguage::Java4, TargetLanguage::Kotlin, MigrationStrategy::Incremental);
        let listed = e.list_plans();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].1, "Alpha");
        assert_eq!(listed[1].1, "Beta");
    }

    #[test]
    fn test_engine_get_plan() {
        let mut e = MigrationEngine::new();
        let plan = e.create_plan("Find Me", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let id = plan.id.clone();
        assert!(e.get_plan(&id).is_some());
        assert_eq!(e.get_plan(&id).unwrap().title, "Find Me");
    }

    #[test]
    fn test_engine_get_plan_not_found() {
        let e = MigrationEngine::new();
        assert!(e.get_plan("nonexistent").is_none());
    }

    #[test]
    fn test_engine_add_translation_rule() {
        let mut e = MigrationEngine::new();
        e.add_translation_rule(TranslationRule::new(
            SourceLanguage::Cobol, TargetLanguage::Rust, "PERFORM", "fn",
        ));
        assert_eq!(e.translation_rules.len(), 1);
    }

    #[test]
    fn test_engine_load_default_rules_cobol_rust() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Cobol, &TargetLanguage::Rust);
        assert!(e.translation_rules.len() >= 4);
    }

    #[test]
    fn test_engine_load_default_rules_java_kotlin() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Java5, &TargetLanguage::Kotlin);
        assert!(e.translation_rules.len() >= 3);
    }

    #[test]
    fn test_engine_load_default_rules_vb6_csharp() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::VB6, &TargetLanguage::CSharp12);
        assert!(e.translation_rules.len() >= 2);
    }

    #[test]
    fn test_engine_load_default_rules_fortran_python() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Fortran, &TargetLanguage::Python);
        assert!(e.translation_rules.len() >= 2);
    }

    #[test]
    fn test_engine_load_default_rules_perl_python() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Perl, &TargetLanguage::Python);
        assert!(e.translation_rules.len() >= 1);
    }

    #[test]
    fn test_engine_load_default_rules_unknown_pair() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Mumps, &TargetLanguage::Swift);
        assert!(!e.translation_rules.is_empty()); // fallback rule
    }

    // --- Analyze codebase ---

    #[test]
    fn test_analyze_empty() {
        let e = MigrationEngine::new();
        let result = e.analyze_codebase(&[]);
        assert_eq!(result.total_components, 0);
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.avg_complexity, 0.0);
    }

    #[test]
    fn test_analyze_single_component() {
        let e = MigrationEngine::new();
        let c = make_component("main", SourceLanguage::Cobol, 1000, 20);
        let result = e.analyze_codebase(&[c]);
        assert_eq!(result.total_components, 1);
        assert_eq!(result.total_lines, 1000);
        assert_eq!(result.avg_complexity, 20.0);
    }

    #[test]
    fn test_analyze_language_breakdown() {
        let e = MigrationEngine::new();
        let c1 = make_component("a", SourceLanguage::Cobol, 500, 10);
        let c2 = make_component("b", SourceLanguage::Cobol, 300, 10);
        let c3 = make_component("c", SourceLanguage::Fortran, 200, 10);
        let result = e.analyze_codebase(&[c1, c2, c3]);
        assert_eq!(result.language_breakdown.get("COBOL"), Some(&2));
        assert_eq!(result.language_breakdown.get("Fortran"), Some(&1));
    }

    #[test]
    fn test_analyze_component_type_breakdown() {
        let e = MigrationEngine::new();
        let c1 = LegacyComponent::new("a", ComponentType::Module, SourceLanguage::Cobol);
        let c2 = LegacyComponent::new("b", ComponentType::BatchJob, SourceLanguage::Cobol);
        let result = e.analyze_codebase(&[c1, c2]);
        assert_eq!(result.component_type_breakdown.get("Module"), Some(&1));
        assert_eq!(result.component_type_breakdown.get("BatchJob"), Some(&1));
    }

    #[test]
    fn test_analyze_generates_warnings_high_complexity() {
        let e = MigrationEngine::new();
        let c = make_component("complex", SourceLanguage::Cobol, 100, 70);
        let result = e.analyze_codebase(&[c]);
        assert!(result.warnings.iter().any(|w| w.contains("complexity")));
    }

    #[test]
    fn test_analyze_generates_warnings_large_codebase() {
        let e = MigrationEngine::new();
        let c = make_component("huge", SourceLanguage::Cobol, 600_000, 10);
        let result = e.analyze_codebase(&[c]);
        assert!(result.warnings.iter().any(|w| w.contains("large codebase")));
    }

    // --- Strategy suggestion ---

    #[test]
    fn test_suggest_big_bang() {
        let e = MigrationEngine::new();
        let mut a = AnalysisResult::new();
        a.total_components = 5;
        a.avg_complexity = 10.0;
        a.high_risk_count = 0;
        assert_eq!(e.suggest_strategy(&a), MigrationStrategy::BigBang);
    }

    #[test]
    fn test_suggest_strangler_fig() {
        let e = MigrationEngine::new();
        let mut a = AnalysisResult::new();
        a.total_components = 100;
        a.avg_complexity = 45.0;
        a.high_risk_count = 10;
        assert_eq!(e.suggest_strategy(&a), MigrationStrategy::StranglerFig);
    }

    #[test]
    fn test_suggest_direct_translation() {
        let e = MigrationEngine::new();
        let mut a = AnalysisResult::new();
        a.total_components = 30;
        a.avg_complexity = 15.0;
        a.high_risk_count = 0;
        assert_eq!(e.suggest_strategy(&a), MigrationStrategy::DirectTranslation);
    }

    // --- Service boundary detection ---

    #[test]
    fn test_identify_service_boundaries_empty() {
        let e = MigrationEngine::new();
        let boundaries = e.identify_service_boundaries(&[]);
        assert!(boundaries.is_empty());
    }

    #[test]
    fn test_identify_service_boundaries_no_data_deps() {
        let e = MigrationEngine::new();
        let c = make_component("a", SourceLanguage::Cobol, 100, 10);
        let boundaries = e.identify_service_boundaries(&[c]);
        assert_eq!(boundaries.len(), 1);
        assert!(boundaries[0].name.contains("core"));
    }

    #[test]
    fn test_identify_service_boundaries_groups_by_data() {
        let e = MigrationEngine::new();
        let mut c1 = make_component("a", SourceLanguage::Cobol, 100, 10);
        c1.add_dependency(make_dep("db-customers", DependencyType::DatabaseAccess, false));
        let mut c2 = make_component("b", SourceLanguage::Cobol, 200, 10);
        c2.add_dependency(make_dep("db-customers", DependencyType::DatabaseAccess, false));
        let mut c3 = make_component("c", SourceLanguage::Cobol, 150, 10);
        c3.add_dependency(make_dep("db-orders", DependencyType::DatabaseAccess, false));
        let boundaries = e.identify_service_boundaries(&[c1, c2, c3]);
        assert_eq!(boundaries.len(), 2); // two db groups
    }

    // --- Translate component ---

    #[test]
    fn test_translate_component_no_rules() {
        let e = MigrationEngine::new();
        let c = make_component("mod", SourceLanguage::Cobol, 1000, 20);
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert_eq!(result.confidence_score, 0.3); // no matching rules
        assert_eq!(result.source_lines, 1000);
    }

    #[test]
    fn test_translate_component_with_rules() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Cobol, &TargetLanguage::Rust);
        let mut c = make_component("mod", SourceLanguage::Cobol, 1000, 20);
        c.file_path = PathBuf::from("/src/main.cbl");
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert!(result.confidence_score > 0.3);
        assert!(!result.rules_applied.is_empty());
        assert_eq!(result.target_path, PathBuf::from("/src/main.rs"));
    }

    #[test]
    fn test_translate_cobol_line_reduction() {
        let e = MigrationEngine::new();
        let mut c = make_component("mod", SourceLanguage::Cobol, 1000, 10);
        c.file_path = PathBuf::from("test.cbl");
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert!(result.target_lines < result.source_lines);
        assert_eq!(result.target_lines, 600); // 60% for COBOL
    }

    #[test]
    fn test_translate_fortran_line_reduction() {
        let e = MigrationEngine::new();
        let mut c = make_component("mod", SourceLanguage::Fortran, 1000, 10);
        c.file_path = PathBuf::from("test.f90");
        let result = e.translate_component(&c, &TargetLanguage::Python);
        assert_eq!(result.target_lines, 700); // 70% for Fortran
    }

    #[test]
    fn test_translate_high_risk_warning() {
        let e = MigrationEngine::new();
        let mut c = make_component("risky", SourceLanguage::Cobol, 100, 10);
        c.risk_level = RiskLevel::High;
        c.file_path = PathBuf::from("test.cbl");
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert!(result.warnings.iter().any(|w| w.contains("High-risk")));
    }

    #[test]
    fn test_translate_over_threshold_warning() {
        let e = MigrationEngine::new();
        let mut c = make_component("complex", SourceLanguage::Cobol, 100, 90);
        c.file_path = PathBuf::from("test.cbl");
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert!(result.warnings.iter().any(|w| w.contains("exceeds threshold")));
    }

    #[test]
    fn test_translate_review_items_from_rules() {
        let mut e = MigrationEngine::new();
        e.add_translation_rule(
            TranslationRule::new(SourceLanguage::Cobol, TargetLanguage::Rust, "GOTO", "// TODO: refactor goto")
                .needs_review()
        );
        let mut c = make_component("mod", SourceLanguage::Cobol, 100, 10);
        c.file_path = PathBuf::from("test.cbl");
        let result = e.translate_component(&c, &TargetLanguage::Rust);
        assert_eq!(result.manual_review_needed.len(), 1);
    }

    // --- Report generation ---

    #[test]
    fn test_generate_report_no_plan() {
        let e = MigrationEngine::new();
        assert!(e.generate_report("nonexistent").is_none());
    }

    #[test]
    fn test_generate_report_empty_plan() {
        let mut e = MigrationEngine::new();
        let plan = e.create_plan("Empty", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let id = plan.id.clone();
        let report = e.generate_report(&id).unwrap();
        assert_eq!(report.components_total, 0);
        assert_eq!(report.lines_source, 0);
        assert_eq!(report.overall_confidence, 0.0);
    }

    #[test]
    fn test_generate_report_with_components() {
        let mut e = MigrationEngine::new();
        e.load_default_rules(&SourceLanguage::Cobol, &TargetLanguage::Rust);
        let plan = e.create_plan("Full", SourceLanguage::Cobol, TargetLanguage::Rust, MigrationStrategy::BigBang);
        let mut c = make_component("main", SourceLanguage::Cobol, 2000, 30);
        c.file_path = PathBuf::from("/src/main.cbl");
        plan.add_component(c);
        let id = plan.id.clone();
        let report = e.generate_report(&id).unwrap();
        assert_eq!(report.components_total, 1);
        assert_eq!(report.lines_source, 2000);
        assert!(report.overall_confidence > 0.0);
        assert!(report.lines_target < report.lines_source);
    }

    // --- Supported pairs ---

    #[test]
    fn test_supported_pairs_not_empty() {
        let pairs = MigrationEngine::supported_pairs();
        assert!(pairs.len() >= 20);
    }

    #[test]
    fn test_supported_pairs_contains_cobol_rust() {
        let pairs = MigrationEngine::supported_pairs();
        assert!(pairs.iter().any(|(s, t)| *s == SourceLanguage::Cobol && *t == TargetLanguage::Rust));
    }

    #[test]
    fn test_supported_pairs_contains_java_kotlin() {
        let pairs = MigrationEngine::supported_pairs();
        assert!(pairs.iter().any(|(s, t)| *s == SourceLanguage::Java5 && *t == TargetLanguage::Kotlin));
    }

    // --- Config defaults ---

    #[test]
    fn test_default_config() {
        let cfg = MigrationConfig::default();
        assert_eq!(cfg.max_complexity_threshold, 80);
        assert!(cfg.auto_service_decomposition);
        assert!(cfg.generate_tests);
        assert!(cfg.preserve_comments);
        assert!(cfg.add_migration_markers);
        assert!(cfg.parallel_translation);
        assert_eq!(cfg.target_test_coverage, 80.0);
    }

    // --- Edge cases ---

    #[test]
    fn test_component_zero_lines() {
        let c = make_component("empty", SourceLanguage::Cobol, 0, 1);
        assert_eq!(c.lines_of_code, 0);
    }

    #[test]
    fn test_custom_source_language() {
        let c = LegacyComponent::new("custom", ComponentType::Module, SourceLanguage::Custom("Natural".to_string()));
        assert_eq!(c.source_language, SourceLanguage::Custom("Natural".to_string()));
    }

    #[test]
    fn test_custom_target_language() {
        let r = TranslationRule::new(
            SourceLanguage::Cobol,
            TargetLanguage::Custom("Zig".to_string()),
            "PERFORM",
            "fn",
        );
        assert_eq!(r.target_lang, TargetLanguage::Custom("Zig".to_string()));
    }

    #[test]
    fn test_migration_status_failed_message() {
        let s = MigrationStatus::Failed("timeout".to_string());
        if let MigrationStatus::Failed(msg) = s {
            assert_eq!(msg, "timeout");
        } else {
            panic!("expected Failed");
        }
    }

    #[test]
    fn test_data_structure_empty_fields() {
        let ds = DataStructure {
            name: "EMPTY-REC".to_string(),
            fields: vec![],
            source: "COPYBOOK".to_string(),
        };
        assert!(ds.fields.is_empty());
    }

    #[test]
    fn test_manual_review_item() {
        let item = ManualReviewItem {
            line: 42,
            original: "GOTO LABEL".to_string(),
            translated: "// TODO goto".to_string(),
            reason: "Goto cannot be directly translated".to_string(),
            suggestion: "Refactor to loop or match".to_string(),
        };
        assert_eq!(item.line, 42);
    }

    #[test]
    fn test_risk_item_probability_and_impact() {
        let ri = RiskItem {
            description: "Data loss".to_string(),
            risk_level: RiskLevel::Critical,
            probability: 0.3,
            impact: 0.9,
            mitigation: "Backup first".to_string(),
        };
        assert!(ri.probability < ri.impact);
    }

    #[test]
    fn test_service_boundary_struct() {
        let sb = ServiceBoundary {
            name: "order-service".to_string(),
            components: vec!["c1".to_string(), "c2".to_string()],
            api_surface: vec!["POST /orders".to_string()],
            data_stores: vec!["orders_db".to_string()],
            estimated_size: 5000,
        };
        assert_eq!(sb.components.len(), 2);
        assert_eq!(sb.estimated_size, 5000);
    }

    #[test]
    fn test_default_rules_cobol_java() {
        let rules = MigrationEngine::default_rules_for_pair(&SourceLanguage::Cobol, &TargetLanguage::Java21);
        assert!(!rules.is_empty());
        assert!(rules.iter().any(|r| r.source_pattern == "PERFORM"));
    }

    #[test]
    fn test_default_rules_csharp_legacy() {
        let rules = MigrationEngine::default_rules_for_pair(&SourceLanguage::CSharpLegacy, &TargetLanguage::CSharp12);
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_target_extension_mapping() {
        assert_eq!(target_extension(&TargetLanguage::Rust), "rs");
        assert_eq!(target_extension(&TargetLanguage::Go), "go");
        assert_eq!(target_extension(&TargetLanguage::Python), "py");
        assert_eq!(target_extension(&TargetLanguage::TypeScript), "ts");
        assert_eq!(target_extension(&TargetLanguage::Kotlin), "kt");
        assert_eq!(target_extension(&TargetLanguage::Custom("Zig".to_string())), "txt");
    }

    #[test]
    fn test_source_language_name_mapping() {
        assert_eq!(source_language_name(&SourceLanguage::Cobol), "COBOL");
        assert_eq!(source_language_name(&SourceLanguage::Fortran), "Fortran");
        assert_eq!(source_language_name(&SourceLanguage::Custom("Ada".to_string())), "Ada");
    }
}
