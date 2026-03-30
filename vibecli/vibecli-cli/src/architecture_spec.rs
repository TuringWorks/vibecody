//! Enterprise Architecture Specification Module
//!
//! Implements four industry-standard architecture frameworks plus governance:
//!
//! | Framework      | Purpose                                    |
//! |----------------|--------------------------------------------|
//! | TOGAF ADM      | Architecture Development Method (9 phases) |
//! | Zachman        | 6x6 classification matrix                  |
//! | C4 Model       | Hierarchical software architecture views   |
//! | ADR            | Architecture Decision Records              |
//! | Governance     | Automated compliance rule evaluation       |
//!
//! All frameworks are unified under `ArchitectureSpec` for holistic
//! enterprise architecture management with JSON export and reporting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TOGAF ADM (Architecture Development Method)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// TOGAF Architecture Development Method phases (ADM cycle).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TogafPhase {
    Preliminary,
    ArchitectureVision,
    BusinessArchitecture,
    InformationSystems,
    TechnologyArchitecture,
    OpportunitiesAndSolutions,
    MigrationPlanning,
    ImplementationGovernance,
    ArchitectureChangeManagement,
}

impl TogafPhase {
    pub fn all() -> &'static [TogafPhase] {
        &[
            Self::Preliminary,
            Self::ArchitectureVision,
            Self::BusinessArchitecture,
            Self::InformationSystems,
            Self::TechnologyArchitecture,
            Self::OpportunitiesAndSolutions,
            Self::MigrationPlanning,
            Self::ImplementationGovernance,
            Self::ArchitectureChangeManagement,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Preliminary => "Preliminary",
            Self::ArchitectureVision => "A: Architecture Vision",
            Self::BusinessArchitecture => "B: Business Architecture",
            Self::InformationSystems => "C: Information Systems",
            Self::TechnologyArchitecture => "D: Technology Architecture",
            Self::OpportunitiesAndSolutions => "E: Opportunities & Solutions",
            Self::MigrationPlanning => "F: Migration Planning",
            Self::ImplementationGovernance => "G: Implementation Governance",
            Self::ArchitectureChangeManagement => "H: Architecture Change Management",
        }
    }

    /// Returns the index in the ADM cycle (0-based).
    pub fn order(&self) -> usize {
        match self {
            Self::Preliminary => 0,
            Self::ArchitectureVision => 1,
            Self::BusinessArchitecture => 2,
            Self::InformationSystems => 3,
            Self::TechnologyArchitecture => 4,
            Self::OpportunitiesAndSolutions => 5,
            Self::MigrationPlanning => 6,
            Self::ImplementationGovernance => 7,
            Self::ArchitectureChangeManagement => 8,
        }
    }

    /// Required artifact types for a phase to be considered minimally complete.
    pub fn required_artifact_types(&self) -> Vec<&str> {
        match self {
            Self::Preliminary => vec!["Architecture Principles", "Stakeholder Map"],
            Self::ArchitectureVision => vec!["Vision Document", "Stakeholder Map", "Value Chain"],
            Self::BusinessArchitecture => {
                vec!["Business Process Catalog", "Organization Map"]
            }
            Self::InformationSystems => {
                vec!["Data Entity Catalog", "Application Portfolio"]
            }
            Self::TechnologyArchitecture => {
                vec!["Technology Standards", "Platform Decomposition"]
            }
            Self::OpportunitiesAndSolutions => {
                vec!["Consolidated Gaps", "Project List"]
            }
            Self::MigrationPlanning => vec!["Migration Plan", "Transition Architecture"],
            Self::ImplementationGovernance => {
                vec!["Compliance Assessment", "Architecture Contract"]
            }
            Self::ArchitectureChangeManagement => {
                vec!["Change Request Log", "Lessons Learned"]
            }
        }
    }
}

impl fmt::Display for TogafPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Type of TOGAF artifact (catalogs, matrices, or diagrams).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactType {
    Catalog,
    Matrix,
    Diagram,
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog => write!(f, "Catalog"),
            Self::Matrix => write!(f, "Matrix"),
            Self::Diagram => write!(f, "Diagram"),
        }
    }
}

/// Status of a TOGAF artifact in the review lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactStatus {
    Draft,
    Review,
    Approved,
    Deprecated,
}

impl fmt::Display for ArtifactStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::Review => write!(f, "Review"),
            Self::Approved => write!(f, "Approved"),
            Self::Deprecated => write!(f, "Deprecated"),
        }
    }
}

/// A TOGAF architecture artifact (catalog, matrix, or diagram).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TogafArtifact {
    pub id: String,
    pub name: String,
    pub phase: TogafPhase,
    pub artifact_type: ArtifactType,
    pub description: String,
    pub content: String,
    pub status: ArtifactStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Vec<String>,
}

impl TogafArtifact {
    pub fn new(
        name: &str,
        phase: TogafPhase,
        artifact_type: ArtifactType,
        description: &str,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_id("togaf"),
            name: name.to_string(),
            phase,
            artifact_type,
            description: description.to_string(),
            content: String::new(),
            status: ArtifactStatus::Draft,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn with_status(mut self, status: ArtifactStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// TOGAF Architecture Development Method manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TogafAdm {
    pub artifacts: Vec<TogafArtifact>,
    pub principles: Vec<ArchitecturePrinciple>,
}

/// An architecture principle (used in Preliminary phase).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturePrinciple {
    pub id: String,
    pub name: String,
    pub statement: String,
    pub rationale: String,
    pub implications: Vec<String>,
}

impl TogafAdm {
    pub fn new() -> Self {
        Self {
            artifacts: Vec::new(),
            principles: Vec::new(),
        }
    }

    /// Add an artifact and return its id.
    pub fn add_artifact(&mut self, artifact: TogafArtifact) -> String {
        let id = artifact.id.clone();
        self.artifacts.push(artifact);
        id
    }

    /// Add an architecture principle.
    pub fn add_principle(&mut self, principle: ArchitecturePrinciple) -> String {
        let id = principle.id.clone();
        self.principles.push(principle);
        id
    }

    /// Get all artifacts belonging to a phase.
    pub fn get_artifacts_by_phase(&self, phase: &TogafPhase) -> Vec<&TogafArtifact> {
        self.artifacts.iter().filter(|a| &a.phase == phase).collect()
    }

    /// Completion percentage for a phase (0.0 - 1.0).
    /// Based on ratio of approved artifacts to required artifact types.
    pub fn get_phase_completion(&self, phase: &TogafPhase) -> f64 {
        let required = phase.required_artifact_types();
        if required.is_empty() {
            return 1.0;
        }
        let phase_artifacts = self.get_artifacts_by_phase(phase);
        let approved_count = phase_artifacts
            .iter()
            .filter(|a| a.status == ArtifactStatus::Approved)
            .count();
        let ratio = approved_count as f64 / required.len() as f64;
        if ratio > 1.0 { 1.0 } else { ratio }
    }

    /// Overall ADM progress across all phases.
    pub fn get_overall_progress(&self) -> f64 {
        let phases = TogafPhase::all();
        let total: f64 = phases.iter().map(|p| self.get_phase_completion(p)).sum();
        total / phases.len() as f64
    }

    /// Generate a textual report for a phase.
    pub fn generate_phase_report(&self, phase: &TogafPhase) -> String {
        let artifacts = self.get_artifacts_by_phase(phase);
        let completion = self.get_phase_completion(phase);
        let prerequisites = self.validate_phase_prerequisites(phase);

        let mut report = String::new();
        report.push_str(&format!("# TOGAF Phase: {}\n\n", phase.label()));
        report.push_str(&format!("Completion: {:.0}%\n\n", completion * 100.0));

        if !prerequisites.is_empty() {
            report.push_str("## Missing Prerequisites\n");
            for p in &prerequisites {
                report.push_str(&format!("- {}\n", p));
            }
            report.push('\n');
        }

        report.push_str("## Artifacts\n");
        if artifacts.is_empty() {
            report.push_str("No artifacts yet.\n");
        } else {
            report.push_str("| Name | Type | Status |\n");
            report.push_str("|------|------|--------|\n");
            for a in &artifacts {
                report.push_str(&format!(
                    "| {} | {} | {} |\n",
                    a.name, a.artifact_type, a.status
                ));
            }
        }

        report
    }

    /// Validate that required artifacts exist for a phase.
    /// Returns a list of missing artifact descriptions.
    pub fn validate_phase_prerequisites(&self, phase: &TogafPhase) -> Vec<String> {
        let required = phase.required_artifact_types();
        let phase_artifacts = self.get_artifacts_by_phase(phase);
        let artifact_names: Vec<&str> = phase_artifacts.iter().map(|a| a.name.as_str()).collect();

        let mut missing = Vec::new();
        for req in required {
            if !artifact_names.iter().any(|n| n.contains(req)) {
                missing.push(format!(
                    "Phase '{}' requires artifact: {}",
                    phase.label(),
                    req
                ));
            }
        }
        missing
    }

    /// Get all artifacts with a given status.
    pub fn get_artifacts_by_status(&self, status: &ArtifactStatus) -> Vec<&TogafArtifact> {
        self.artifacts.iter().filter(|a| &a.status == status).collect()
    }

    /// Update artifact status by id. Returns true if found.
    pub fn update_artifact_status(&mut self, id: &str, status: ArtifactStatus) -> bool {
        if let Some(art) = self.artifacts.iter_mut().find(|a| a.id == id) {
            art.status = status;
            art.updated_at = current_timestamp();
            true
        } else {
            false
        }
    }
}

impl Default for TogafAdm {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Zachman Framework
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Zachman Framework row perspectives (rows of the 6x6 matrix).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZachmanPerspective {
    /// Executive / scope (contextual)
    Planner,
    /// Business management (conceptual)
    Owner,
    /// Architect (logical)
    Designer,
    /// Engineer (physical)
    Builder,
    /// Technician (detailed representations)
    Implementer,
    /// Enterprise end user (functioning enterprise)
    Worker,
}

impl ZachmanPerspective {
    pub fn all() -> &'static [ZachmanPerspective] {
        &[
            Self::Planner,
            Self::Owner,
            Self::Designer,
            Self::Builder,
            Self::Implementer,
            Self::Worker,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Planner => "Planner (Scope)",
            Self::Owner => "Owner (Business)",
            Self::Designer => "Designer (System)",
            Self::Builder => "Builder (Technology)",
            Self::Implementer => "Implementer (Detail)",
            Self::Worker => "Worker (Enterprise)",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Planner => 0,
            Self::Owner => 1,
            Self::Designer => 2,
            Self::Builder => 3,
            Self::Implementer => 4,
            Self::Worker => 5,
        }
    }
}

impl fmt::Display for ZachmanPerspective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Zachman Framework column aspects (columns of the 6x6 matrix).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZachmanAspect {
    /// What (Data / Inventory sets)
    What,
    /// How (Function / Process flows)
    How,
    /// Where (Network / Distribution)
    Where,
    /// Who (People / Responsibility)
    Who,
    /// When (Time / Dynamics)
    When,
    /// Why (Motivation / Ends and means)
    Why,
}

impl ZachmanAspect {
    pub fn all() -> &'static [ZachmanAspect] {
        &[
            Self::What,
            Self::How,
            Self::Where,
            Self::Who,
            Self::When,
            Self::Why,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            Self::What => "What (Data)",
            Self::How => "How (Function)",
            Self::Where => "Where (Network)",
            Self::Who => "Who (People)",
            Self::When => "When (Time)",
            Self::Why => "Why (Motivation)",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::What => 0,
            Self::How => 1,
            Self::Where => 2,
            Self::Who => 3,
            Self::When => 4,
            Self::Why => 5,
        }
    }
}

impl fmt::Display for ZachmanAspect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A single cell in the Zachman 6x6 matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZachmanCell {
    pub perspective: ZachmanPerspective,
    pub aspect: ZachmanAspect,
    pub content: String,
    pub artifacts: Vec<String>,
    /// Maturity level 0 (empty) to 5 (optimized).
    pub maturity: u8,
}

impl ZachmanCell {
    pub fn new(perspective: ZachmanPerspective, aspect: ZachmanAspect) -> Self {
        Self {
            perspective,
            aspect,
            content: String::new(),
            artifacts: Vec::new(),
            maturity: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty() && self.artifacts.is_empty()
    }
}

/// Zachman Framework 6x6 classification matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZachmanFramework {
    cells: HashMap<String, ZachmanCell>,
}

impl ZachmanFramework {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    fn cell_key(perspective: &ZachmanPerspective, aspect: &ZachmanAspect) -> String {
        format!("{}:{}", perspective.index(), aspect.index())
    }

    /// Set content for a cell. Creates the cell if it does not exist.
    pub fn set_cell(
        &mut self,
        perspective: ZachmanPerspective,
        aspect: ZachmanAspect,
        content: &str,
    ) {
        let key = Self::cell_key(&perspective, &aspect);
        let cell = self.cells.entry(key).or_insert_with(|| {
            ZachmanCell::new(perspective.clone(), aspect.clone())
        });
        cell.content = content.to_string();
        if cell.maturity == 0 && !content.is_empty() {
            cell.maturity = 1;
        }
    }

    /// Set maturity level for a cell (0-5).
    pub fn set_cell_maturity(
        &mut self,
        perspective: &ZachmanPerspective,
        aspect: &ZachmanAspect,
        maturity: u8,
    ) -> bool {
        let key = Self::cell_key(perspective, aspect);
        if let Some(cell) = self.cells.get_mut(&key) {
            cell.maturity = maturity.min(5);
            true
        } else {
            false
        }
    }

    /// Add an artifact reference to a cell.
    pub fn add_cell_artifact(
        &mut self,
        perspective: &ZachmanPerspective,
        aspect: &ZachmanAspect,
        artifact: &str,
    ) {
        let key = Self::cell_key(perspective, aspect);
        let cell = self.cells.entry(key).or_insert_with(|| {
            ZachmanCell::new(perspective.clone(), aspect.clone())
        });
        cell.artifacts.push(artifact.to_string());
    }

    /// Get a cell by perspective and aspect.
    pub fn get_cell(
        &self,
        perspective: &ZachmanPerspective,
        aspect: &ZachmanAspect,
    ) -> Option<&ZachmanCell> {
        let key = Self::cell_key(perspective, aspect);
        self.cells.get(&key)
    }

    /// Percentage of non-empty cells (0.0 - 1.0).
    pub fn get_coverage(&self) -> f64 {
        let total = 36; // 6x6
        let filled = self
            .cells
            .values()
            .filter(|c| !c.is_empty())
            .count();
        filled as f64 / total as f64
    }

    /// Return (perspective, aspect) pairs for empty cells.
    pub fn get_gaps(&self) -> Vec<(ZachmanPerspective, ZachmanAspect)> {
        let mut gaps = Vec::new();
        for p in ZachmanPerspective::all() {
            for a in ZachmanAspect::all() {
                let key = Self::cell_key(p, a);
                let is_empty = match self.cells.get(&key) {
                    Some(c) => c.is_empty(),
                    None => true,
                };
                if is_empty {
                    gaps.push((p.clone(), a.clone()));
                }
            }
        }
        gaps
    }

    /// Average maturity across all filled cells.
    pub fn average_maturity(&self) -> f64 {
        let filled: Vec<&ZachmanCell> = self.cells.values().filter(|c| !c.is_empty()).collect();
        if filled.is_empty() {
            return 0.0;
        }
        let total: u32 = filled.iter().map(|c| c.maturity as u32).sum();
        total as f64 / filled.len() as f64
    }

    /// Generate a formatted 6x6 matrix report.
    pub fn generate_matrix_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Zachman Framework Matrix\n\n");

        // Header row
        report.push_str("| Perspective |");
        for a in ZachmanAspect::all() {
            report.push_str(&format!(" {} |", a.label()));
        }
        report.push('\n');
        report.push_str("|-------------|");
        for _ in ZachmanAspect::all() {
            report.push_str("-------------|");
        }
        report.push('\n');

        for p in ZachmanPerspective::all() {
            report.push_str(&format!("| {} |", p.label()));
            for a in ZachmanAspect::all() {
                let key = Self::cell_key(p, a);
                let display = match self.cells.get(&key) {
                    Some(c) if !c.is_empty() => {
                        let truncated = if c.content.len() > 20 {
                            format!("{}...", &c.content[..17])
                        } else {
                            c.content.clone()
                        };
                        format!("{} (M{})", truncated, c.maturity)
                    }
                    _ => "—".to_string(),
                };
                report.push_str(&format!(" {} |", display));
            }
            report.push('\n');
        }

        report.push_str(&format!(
            "\nCoverage: {:.0}% | Avg Maturity: {:.1}/5\n",
            self.get_coverage() * 100.0,
            self.average_maturity()
        ));

        report
    }

    /// Validate cross-cell consistency.
    /// Checks that if a Designer row cell references something, related Builder/Owner cells exist.
    pub fn validate_consistency(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Rule 1: If Designer row has content, Owner row should too
        for a in ZachmanAspect::all() {
            let designer_key = Self::cell_key(&ZachmanPerspective::Designer, a);
            let owner_key = Self::cell_key(&ZachmanPerspective::Owner, a);
            let has_designer = self
                .cells
                .get(&designer_key)
                .map(|c| !c.is_empty())
                .unwrap_or(false);
            let has_owner = self
                .cells
                .get(&owner_key)
                .map(|c| !c.is_empty())
                .unwrap_or(false);
            if has_designer && !has_owner {
                issues.push(format!(
                    "Designer has {} content but Owner row is empty — business context missing",
                    a.label()
                ));
            }
        }

        // Rule 2: If Builder row has content, Designer row should too
        for a in ZachmanAspect::all() {
            let builder_key = Self::cell_key(&ZachmanPerspective::Builder, a);
            let designer_key = Self::cell_key(&ZachmanPerspective::Designer, a);
            let has_builder = self
                .cells
                .get(&builder_key)
                .map(|c| !c.is_empty())
                .unwrap_or(false);
            let has_designer = self
                .cells
                .get(&designer_key)
                .map(|c| !c.is_empty())
                .unwrap_or(false);
            if has_builder && !has_designer {
                issues.push(format!(
                    "Builder has {} content but Designer row is empty — logical model missing",
                    a.label()
                ));
            }
        }

        // Rule 3: Planner row should have at least What and Why
        let planner_what = Self::cell_key(&ZachmanPerspective::Planner, &ZachmanAspect::What);
        let planner_why = Self::cell_key(&ZachmanPerspective::Planner, &ZachmanAspect::Why);
        let has_any_planner = ZachmanAspect::all().iter().any(|a| {
            self.cells
                .get(&Self::cell_key(&ZachmanPerspective::Planner, a))
                .map(|c| !c.is_empty())
                .unwrap_or(false)
        });
        if has_any_planner {
            let missing_what = self
                .cells
                .get(&planner_what)
                .map(|c| c.is_empty())
                .unwrap_or(true);
            let missing_why = self
                .cells
                .get(&planner_why)
                .map(|c| c.is_empty())
                .unwrap_or(true);
            if missing_what {
                issues.push("Planner perspective has content but What (Data) is missing".to_string());
            }
            if missing_why {
                issues.push(
                    "Planner perspective has content but Why (Motivation) is missing".to_string(),
                );
            }
        }

        issues
    }
}

impl Default for ZachmanFramework {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// C4 Model
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// C4 abstraction levels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum C4Level {
    Context,
    Container,
    Component,
    Code,
}

impl C4Level {
    pub fn label(&self) -> &str {
        match self {
            Self::Context => "System Context",
            Self::Container => "Container",
            Self::Component => "Component",
            Self::Code => "Code",
        }
    }
}

impl fmt::Display for C4Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Type of C4 element.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum C4ElementType {
    Person,
    SoftwareSystem,
    Container,
    Component,
}

impl fmt::Display for C4ElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Person => write!(f, "Person"),
            Self::SoftwareSystem => write!(f, "Software System"),
            Self::Container => write!(f, "Container"),
            Self::Component => write!(f, "Component"),
        }
    }
}

/// A C4 model element (person, system, container, or component).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Element {
    pub id: String,
    pub name: String,
    pub element_type: C4ElementType,
    pub description: String,
    pub technology: String,
    pub tags: Vec<String>,
    /// For containers: which system they belong to.
    pub parent_id: Option<String>,
}

impl C4Element {
    pub fn new(
        name: &str,
        element_type: C4ElementType,
        description: &str,
    ) -> Self {
        Self {
            id: generate_id("c4"),
            name: name.to_string(),
            element_type,
            description: description.to_string(),
            technology: String::new(),
            tags: Vec::new(),
            parent_id: None,
        }
    }

    pub fn with_technology(mut self, tech: &str) -> Self {
        self.technology = tech.to_string();
        self
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
}

/// A relationship between two C4 elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Relationship {
    pub source_id: String,
    pub target_id: String,
    pub description: String,
    pub technology: String,
}

impl C4Relationship {
    pub fn new(source_id: &str, target_id: &str, description: &str) -> Self {
        Self {
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            description: description.to_string(),
            technology: String::new(),
        }
    }

    pub fn with_technology(mut self, tech: &str) -> Self {
        self.technology = tech.to_string();
        self
    }
}

/// C4 architecture model with elements and relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C4Model {
    pub system_name: String,
    pub elements: Vec<C4Element>,
    pub relationships: Vec<C4Relationship>,
}

impl C4Model {
    pub fn new(system_name: &str) -> Self {
        Self {
            system_name: system_name.to_string(),
            elements: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Add an element and return its id.
    pub fn add_element(&mut self, element: C4Element) -> String {
        let id = element.id.clone();
        self.elements.push(element);
        id
    }

    /// Add a relationship.
    pub fn add_relationship(&mut self, rel: C4Relationship) {
        self.relationships.push(rel);
    }

    /// Get an element by id.
    pub fn get_element(&self, id: &str) -> Option<&C4Element> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// Get all elements of a given type.
    pub fn get_elements_by_type(&self, element_type: &C4ElementType) -> Vec<&C4Element> {
        self.elements
            .iter()
            .filter(|e| &e.element_type == element_type)
            .collect()
    }

    /// Get child elements of a parent.
    pub fn get_children(&self, parent_id: &str) -> Vec<&C4Element> {
        self.elements
            .iter()
            .filter(|e| e.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    /// Get relationships involving an element.
    pub fn get_relationships_for(&self, element_id: &str) -> Vec<&C4Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_id == element_id || r.target_id == element_id)
            .collect()
    }

    /// Generate a Mermaid system context diagram.
    pub fn generate_context_diagram(&self) -> String {
        let mut mermaid = String::new();
        mermaid.push_str("graph TB\n");
        mermaid.push_str(&format!("    title[\"System Context: {}\"]\n", self.system_name));
        mermaid.push_str("    style title fill:none,stroke:none\n\n");

        // Persons
        for el in self.get_elements_by_type(&C4ElementType::Person) {
            mermaid.push_str(&format!(
                "    {}[\"{}<br/><i>{}</i>\"]\n",
                sanitize_mermaid_id(&el.id),
                el.name,
                el.description
            ));
            mermaid.push_str(&format!(
                "    style {} fill:#08427B,color:#fff\n",
                sanitize_mermaid_id(&el.id)
            ));
        }

        // Software systems
        for el in self.get_elements_by_type(&C4ElementType::SoftwareSystem) {
            mermaid.push_str(&format!(
                "    {}[\"{}<br/><i>{}</i>\"]\n",
                sanitize_mermaid_id(&el.id),
                el.name,
                el.description
            ));
            mermaid.push_str(&format!(
                "    style {} fill:#1168BD,color:#fff\n",
                sanitize_mermaid_id(&el.id)
            ));
        }

        // Relationships between context-level elements
        let context_ids: Vec<&str> = self
            .elements
            .iter()
            .filter(|e| {
                e.element_type == C4ElementType::Person
                    || e.element_type == C4ElementType::SoftwareSystem
            })
            .map(|e| e.id.as_str())
            .collect();

        for rel in &self.relationships {
            if context_ids.contains(&rel.source_id.as_str())
                && context_ids.contains(&rel.target_id.as_str())
            {
                let label = if rel.technology.is_empty() {
                    rel.description.clone()
                } else {
                    format!("{}<br/>[{}]", rel.description, rel.technology)
                };
                mermaid.push_str(&format!(
                    "    {} -->|\"{}\"| {}\n",
                    sanitize_mermaid_id(&rel.source_id),
                    label,
                    sanitize_mermaid_id(&rel.target_id)
                ));
            }
        }

        mermaid
    }

    /// Generate a Mermaid container diagram.
    pub fn generate_container_diagram(&self) -> String {
        let mut mermaid = String::new();
        mermaid.push_str("graph TB\n");
        mermaid.push_str(&format!(
            "    title[\"Container Diagram: {}\"]\n",
            self.system_name
        ));
        mermaid.push_str("    style title fill:none,stroke:none\n\n");

        // Persons
        for el in self.get_elements_by_type(&C4ElementType::Person) {
            mermaid.push_str(&format!(
                "    {}[\"{}<br/><i>Person</i>\"]\n",
                sanitize_mermaid_id(&el.id),
                el.name,
            ));
        }

        // Containers
        for el in self.get_elements_by_type(&C4ElementType::Container) {
            let tech_label = if el.technology.is_empty() {
                String::new()
            } else {
                format!("<br/>[{}]", el.technology)
            };
            mermaid.push_str(&format!(
                "    {}[\"{}{}<br/><i>{}</i>\"]\n",
                sanitize_mermaid_id(&el.id),
                el.name,
                tech_label,
                el.description
            ));
            mermaid.push_str(&format!(
                "    style {} fill:#438DD5,color:#fff\n",
                sanitize_mermaid_id(&el.id)
            ));
        }

        // External systems
        for el in self.get_elements_by_type(&C4ElementType::SoftwareSystem) {
            mermaid.push_str(&format!(
                "    {}[\"{}<br/><i>External</i>\"]\n",
                sanitize_mermaid_id(&el.id),
                el.name,
            ));
            mermaid.push_str(&format!(
                "    style {} fill:#999,color:#fff\n",
                sanitize_mermaid_id(&el.id)
            ));
        }

        // Relationships
        let container_ids: Vec<&str> = self
            .elements
            .iter()
            .filter(|e| {
                e.element_type == C4ElementType::Container
                    || e.element_type == C4ElementType::Person
                    || e.element_type == C4ElementType::SoftwareSystem
            })
            .map(|e| e.id.as_str())
            .collect();

        for rel in &self.relationships {
            if container_ids.contains(&rel.source_id.as_str())
                && container_ids.contains(&rel.target_id.as_str())
            {
                mermaid.push_str(&format!(
                    "    {} -->|\"{}\"| {}\n",
                    sanitize_mermaid_id(&rel.source_id),
                    rel.description,
                    sanitize_mermaid_id(&rel.target_id)
                ));
            }
        }

        mermaid
    }

    /// Generate a Mermaid component diagram for a specific container.
    pub fn generate_component_diagram(&self, container_id: &str) -> String {
        let container_name = self
            .get_element(container_id)
            .map(|e| e.name.as_str())
            .unwrap_or("Unknown");

        let mut mermaid = String::new();
        mermaid.push_str("graph TB\n");
        mermaid.push_str(&format!(
            "    title[\"Component Diagram: {}\"]\n",
            container_name
        ));
        mermaid.push_str("    style title fill:none,stroke:none\n\n");

        let components = self.get_children(container_id);
        let component_ids: Vec<&str> = components.iter().map(|c| c.id.as_str()).collect();

        for comp in &components {
            let tech_label = if comp.technology.is_empty() {
                String::new()
            } else {
                format!("<br/>[{}]", comp.technology)
            };
            mermaid.push_str(&format!(
                "    {}[\"{}{}<br/><i>{}</i>\"]\n",
                sanitize_mermaid_id(&comp.id),
                comp.name,
                tech_label,
                comp.description
            ));
            mermaid.push_str(&format!(
                "    style {} fill:#85BBF0,color:#000\n",
                sanitize_mermaid_id(&comp.id)
            ));
        }

        for rel in &self.relationships {
            let src_match = component_ids.contains(&rel.source_id.as_str());
            let tgt_match = component_ids.contains(&rel.target_id.as_str());
            if src_match || tgt_match {
                // Include the element if at least one endpoint is a component of this container
                mermaid.push_str(&format!(
                    "    {} -->|\"{}\"| {}\n",
                    sanitize_mermaid_id(&rel.source_id),
                    rel.description,
                    sanitize_mermaid_id(&rel.target_id)
                ));
            }
        }

        mermaid
    }

    /// Validate the C4 model for common issues.
    pub fn validate_model(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for orphan elements (no relationships)
        for el in &self.elements {
            let has_rel = self.relationships.iter().any(|r| {
                r.source_id == el.id || r.target_id == el.id
            });
            if !has_rel {
                issues.push(format!(
                    "Orphan element '{}' ({}) has no relationships",
                    el.name, el.element_type
                ));
            }
        }

        // Check for dangling relationship references
        let element_ids: Vec<&str> = self.elements.iter().map(|e| e.id.as_str()).collect();
        for rel in &self.relationships {
            if !element_ids.contains(&rel.source_id.as_str()) {
                issues.push(format!(
                    "Relationship references unknown source: {}",
                    rel.source_id
                ));
            }
            if !element_ids.contains(&rel.target_id.as_str()) {
                issues.push(format!(
                    "Relationship references unknown target: {}",
                    rel.target_id
                ));
            }
        }

        // Check for self-referencing relationships
        for rel in &self.relationships {
            if rel.source_id == rel.target_id {
                issues.push(format!(
                    "Self-referencing relationship on element: {}",
                    rel.source_id
                ));
            }
        }

        // Check containers have parent systems
        for el in &self.elements {
            if el.element_type == C4ElementType::Container && el.parent_id.is_none() {
                issues.push(format!(
                    "Container '{}' has no parent system",
                    el.name
                ));
            }
        }

        // Check components have parent containers
        for el in &self.elements {
            if el.element_type == C4ElementType::Component && el.parent_id.is_none() {
                issues.push(format!(
                    "Component '{}' has no parent container",
                    el.name
                ));
            }
        }

        // Check for missing descriptions
        for el in &self.elements {
            if el.description.is_empty() {
                issues.push(format!(
                    "Element '{}' ({}) has no description",
                    el.name, el.element_type
                ));
            }
        }

        // Check duplicate element names within same type
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        for el in &self.elements {
            let key = (el.name.clone(), format!("{}", el.element_type));
            *seen.entry(key).or_insert(0) += 1;
        }
        for ((name, etype), count) in &seen {
            if *count > 1 {
                issues.push(format!(
                    "Duplicate {} name: '{}' appears {} times",
                    etype, name, count
                ));
            }
        }

        issues
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Architecture Decision Records (ADRs)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// ADR lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    /// Superseded by another ADR (references the new ADR id).
    Superseded(String),
}

impl fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proposed => write!(f, "Proposed"),
            Self::Accepted => write!(f, "Accepted"),
            Self::Deprecated => write!(f, "Deprecated"),
            Self::Superseded(by) => write!(f, "Superseded by {}", by),
        }
    }
}

/// An Architecture Decision Record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub date: String,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub participants: Vec<String>,
    /// ID of the ADR this supersedes (if any).
    pub supersedes: Option<String>,
    pub tags: Vec<String>,
}

impl Adr {
    pub fn new(title: &str, context: &str, decision: &str) -> Self {
        Self {
            id: generate_id("adr"),
            title: title.to_string(),
            status: AdrStatus::Proposed,
            date: "2026-03-29".to_string(),
            context: context.to_string(),
            decision: decision.to_string(),
            consequences: Vec::new(),
            participants: Vec::new(),
            supersedes: None,
            tags: Vec::new(),
        }
    }

    pub fn with_consequences(mut self, consequences: Vec<String>) -> Self {
        self.consequences = consequences;
        self
    }

    pub fn with_participants(mut self, participants: Vec<String>) -> Self {
        self.participants = participants;
        self
    }

    pub fn with_date(mut self, date: &str) -> Self {
        self.date = date.to_string();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
}

/// Store for managing Architecture Decision Records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrStore {
    pub records: Vec<Adr>,
}

impl AdrStore {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Add an ADR and return its id.
    pub fn add(&mut self, adr: Adr) -> String {
        let id = adr.id.clone();
        self.records.push(adr);
        id
    }

    /// Get an ADR by id.
    pub fn get(&self, id: &str) -> Option<&Adr> {
        self.records.iter().find(|a| a.id == id)
    }

    /// Get a mutable reference to an ADR by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Adr> {
        self.records.iter_mut().find(|a| a.id == id)
    }

    /// List all ADRs.
    pub fn list(&self) -> Vec<&Adr> {
        self.records.iter().collect()
    }

    /// Accept an ADR by id.
    pub fn accept(&mut self, id: &str) -> bool {
        if let Some(adr) = self.get_mut(id) {
            adr.status = AdrStatus::Accepted;
            true
        } else {
            false
        }
    }

    /// Deprecate an ADR by id.
    pub fn deprecate(&mut self, id: &str) -> bool {
        if let Some(adr) = self.get_mut(id) {
            adr.status = AdrStatus::Deprecated;
            true
        } else {
            false
        }
    }

    /// Supersede an ADR with a new one.
    pub fn supersede(&mut self, old_id: &str, new_id: &str) -> bool {
        if let Some(adr) = self.get_mut(old_id) {
            adr.status = AdrStatus::Superseded(new_id.to_string());
            true
        } else {
            false
        }
    }

    /// Generate standard ADR markdown for a specific record.
    pub fn generate_markdown(&self, id: &str) -> String {
        match self.get(id) {
            Some(adr) => {
                let mut md = String::new();
                md.push_str(&format!("# ADR: {}\n\n", adr.title));
                md.push_str(&format!("**ID:** {}\n", adr.id));
                md.push_str(&format!("**Date:** {}\n", adr.date));
                md.push_str(&format!("**Status:** {}\n\n", adr.status));

                if !adr.participants.is_empty() {
                    md.push_str(&format!(
                        "**Participants:** {}\n\n",
                        adr.participants.join(", ")
                    ));
                }

                if let Some(ref sup) = adr.supersedes {
                    md.push_str(&format!("**Supersedes:** {}\n\n", sup));
                }

                md.push_str("## Context\n\n");
                md.push_str(&adr.context);
                md.push_str("\n\n## Decision\n\n");
                md.push_str(&adr.decision);
                md.push_str("\n\n## Consequences\n\n");

                if adr.consequences.is_empty() {
                    md.push_str("_No consequences documented._\n");
                } else {
                    for c in &adr.consequences {
                        md.push_str(&format!("- {}\n", c));
                    }
                }

                if !adr.tags.is_empty() {
                    md.push_str(&format!("\n**Tags:** {}\n", adr.tags.join(", ")));
                }

                md
            }
            None => format!("ADR '{}' not found.\n", id),
        }
    }

    /// Generate an index table of all ADRs.
    pub fn generate_index(&self) -> String {
        let mut index = String::new();
        index.push_str("# Architecture Decision Records\n\n");
        index.push_str("| ID | Title | Status | Date |\n");
        index.push_str("|----|-------|--------|------|\n");
        for adr in &self.records {
            index.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                adr.id, adr.title, adr.status, adr.date
            ));
        }
        index.push_str(&format!("\nTotal: {} records\n", self.records.len()));
        index
    }

    /// Search ADRs by text query (matches title, context, decision, tags).
    pub fn search(&self, query: &str) -> Vec<&Adr> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .filter(|adr| {
                adr.title.to_lowercase().contains(&q)
                    || adr.context.to_lowercase().contains(&q)
                    || adr.decision.to_lowercase().contains(&q)
                    || adr.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Get ADRs by status.
    pub fn get_by_status(&self, status: &AdrStatus) -> Vec<&Adr> {
        self.records.iter().filter(|a| &a.status == status).collect()
    }

    /// Get ADR count by status.
    pub fn status_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for adr in &self.records {
            let key = match &adr.status {
                AdrStatus::Proposed => "Proposed".to_string(),
                AdrStatus::Accepted => "Accepted".to_string(),
                AdrStatus::Deprecated => "Deprecated".to_string(),
                AdrStatus::Superseded(_) => "Superseded".to_string(),
            };
            *summary.entry(key).or_insert(0) += 1;
        }
        summary
    }
}

impl Default for AdrStore {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Architecture Governance
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Severity of a governance rule or violation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GovernanceSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for GovernanceSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A governance rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Human-readable description of what the check does.
    pub check_fn_description: String,
    pub severity: GovernanceSeverity,
    /// Category for grouping (e.g. "security", "consistency", "completeness")
    pub category: String,
}

impl GovernanceRule {
    pub fn new(name: &str, description: &str, severity: GovernanceSeverity) -> Self {
        Self {
            id: generate_id("gov"),
            name: name.to_string(),
            description: description.to_string(),
            check_fn_description: String::new(),
            severity,
            category: "general".to_string(),
        }
    }

    pub fn with_check_description(mut self, desc: &str) -> Self {
        self.check_fn_description = desc.to_string();
        self
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
}

/// A governance violation found during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceViolation {
    pub rule_id: String,
    pub rule_name: String,
    pub message: String,
    pub severity: GovernanceSeverity,
    pub recommendation: String,
}

impl GovernanceViolation {
    pub fn new(
        rule_id: &str,
        rule_name: &str,
        message: &str,
        severity: GovernanceSeverity,
        recommendation: &str,
    ) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            rule_name: rule_name.to_string(),
            message: message.to_string(),
            severity,
            recommendation: recommendation.to_string(),
        }
    }
}

/// Governance engine that evaluates architecture compliance rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEngine {
    pub rules: Vec<GovernanceRule>,
}

impl GovernanceEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create an engine with standard enterprise architecture rules.
    pub fn with_standard_rules() -> Self {
        let mut engine = Self::new();

        engine.add_rule(
            GovernanceRule::new(
                "C4 Model Completeness",
                "All C4 elements must have descriptions and relationships",
                GovernanceSeverity::Error,
            )
            .with_check_description("Validates no orphan elements and all have descriptions")
            .with_category("completeness")
            .with_id("GOV-001"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "TOGAF Phase Prerequisites",
                "Each TOGAF phase must have required artifacts before starting next",
                GovernanceSeverity::Warning,
            )
            .with_check_description("Checks each phase has its minimum required artifact types")
            .with_category("process")
            .with_id("GOV-002"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "ADR Coverage",
                "Key architectural decisions must be documented as ADRs",
                GovernanceSeverity::Warning,
            )
            .with_check_description("Ensures at least one accepted ADR exists")
            .with_category("documentation")
            .with_id("GOV-003"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "C4 Technology Stack",
                "Containers should specify their technology",
                GovernanceSeverity::Info,
            )
            .with_check_description("Checks containers have non-empty technology fields")
            .with_category("completeness")
            .with_id("GOV-004"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "TOGAF Artifact Approval",
                "Artifacts should not remain in Draft status indefinitely",
                GovernanceSeverity::Warning,
            )
            .with_check_description("Flags phases with more than 50% draft artifacts")
            .with_category("process")
            .with_id("GOV-005"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "C4 Container Parentage",
                "Containers must belong to a software system",
                GovernanceSeverity::Error,
            )
            .with_check_description("Validates every container has a parent system id")
            .with_category("consistency")
            .with_id("GOV-006"),
        );

        engine.add_rule(
            GovernanceRule::new(
                "ADR Supersession Chain",
                "Superseded ADRs should reference valid successors",
                GovernanceSeverity::Error,
            )
            .with_check_description("Validates supersession references point to existing ADRs")
            .with_category("consistency")
            .with_id("GOV-007"),
        );

        engine
    }

    /// Add a governance rule.
    pub fn add_rule(&mut self, rule: GovernanceRule) {
        self.rules.push(rule);
    }

    /// Get a rule by id.
    pub fn get_rule(&self, id: &str) -> Option<&GovernanceRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Evaluate all governance rules against the given architecture models.
    pub fn evaluate(
        &self,
        model: &C4Model,
        adm: &TogafAdm,
        adrs: &AdrStore,
    ) -> Vec<GovernanceViolation> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            match rule.id.as_str() {
                "GOV-001" => {
                    // C4 Model Completeness
                    let c4_issues = model.validate_model();
                    for issue in c4_issues {
                        violations.push(GovernanceViolation::new(
                            &rule.id,
                            &rule.name,
                            &issue,
                            rule.severity.clone(),
                            "Add missing descriptions or relationships to the C4 model",
                        ));
                    }
                }
                "GOV-002" => {
                    // TOGAF Phase Prerequisites
                    for phase in TogafPhase::all() {
                        let missing = adm.validate_phase_prerequisites(phase);
                        for m in missing {
                            violations.push(GovernanceViolation::new(
                                &rule.id,
                                &rule.name,
                                &m,
                                rule.severity.clone(),
                                "Create the required artifact for this phase",
                            ));
                        }
                    }
                }
                "GOV-003" => {
                    // ADR Coverage
                    let accepted = adrs.get_by_status(&AdrStatus::Accepted);
                    if accepted.is_empty() {
                        violations.push(GovernanceViolation::new(
                            &rule.id,
                            &rule.name,
                            "No accepted ADRs found — key decisions are undocumented",
                            rule.severity.clone(),
                            "Create and accept ADRs for major architectural decisions",
                        ));
                    }
                }
                "GOV-004" => {
                    // C4 Technology Stack
                    for el in model.get_elements_by_type(&C4ElementType::Container) {
                        if el.technology.is_empty() {
                            violations.push(GovernanceViolation::new(
                                &rule.id,
                                &rule.name,
                                &format!(
                                    "Container '{}' has no technology specified",
                                    el.name
                                ),
                                rule.severity.clone(),
                                "Specify the technology (e.g., 'Java/Spring Boot', 'React/TypeScript')",
                            ));
                        }
                    }
                }
                "GOV-005" => {
                    // TOGAF Artifact Approval
                    for phase in TogafPhase::all() {
                        let arts = adm.get_artifacts_by_phase(phase);
                        if arts.is_empty() {
                            continue;
                        }
                        let draft_count = arts
                            .iter()
                            .filter(|a| a.status == ArtifactStatus::Draft)
                            .count();
                        let ratio = draft_count as f64 / arts.len() as f64;
                        if ratio > 0.5 {
                            violations.push(GovernanceViolation::new(
                                &rule.id,
                                &rule.name,
                                &format!(
                                    "Phase '{}' has {:.0}% draft artifacts",
                                    phase.label(),
                                    ratio * 100.0
                                ),
                                rule.severity.clone(),
                                "Review and approve artifacts to advance the architecture cycle",
                            ));
                        }
                    }
                }
                "GOV-006" => {
                    // C4 Container Parentage
                    for el in model.get_elements_by_type(&C4ElementType::Container) {
                        if el.parent_id.is_none() {
                            violations.push(GovernanceViolation::new(
                                &rule.id,
                                &rule.name,
                                &format!(
                                    "Container '{}' has no parent system",
                                    el.name
                                ),
                                rule.severity.clone(),
                                "Set parent_id to the owning SoftwareSystem element",
                            ));
                        }
                    }
                }
                "GOV-007" => {
                    // ADR Supersession Chain
                    for adr in &adrs.records {
                        if let AdrStatus::Superseded(ref new_id) = adr.status {
                            if adrs.get(new_id).is_none() {
                                violations.push(GovernanceViolation::new(
                                    &rule.id,
                                    &rule.name,
                                    &format!(
                                        "ADR '{}' superseded by '{}' which does not exist",
                                        adr.id, new_id
                                    ),
                                    rule.severity.clone(),
                                    "Create the successor ADR or fix the supersession reference",
                                ));
                            }
                        }
                    }
                }
                _ => {
                    // Custom rules — skip automated evaluation
                }
            }
        }

        violations
    }

    /// Generate a governance report.
    pub fn generate_report(
        &self,
        model: &C4Model,
        adm: &TogafAdm,
        adrs: &AdrStore,
    ) -> String {
        let violations = self.evaluate(model, adm, adrs);

        let mut report = String::new();
        report.push_str("# Architecture Governance Report\n\n");
        report.push_str(&format!("Rules: {} | Violations: {}\n\n", self.rules.len(), violations.len()));

        if violations.is_empty() {
            report.push_str("All governance rules pass.\n");
            return report;
        }

        // Group by severity
        let critical: Vec<&GovernanceViolation> = violations
            .iter()
            .filter(|v| v.severity == GovernanceSeverity::Critical)
            .collect();
        let errors: Vec<&GovernanceViolation> = violations
            .iter()
            .filter(|v| v.severity == GovernanceSeverity::Error)
            .collect();
        let warnings: Vec<&GovernanceViolation> = violations
            .iter()
            .filter(|v| v.severity == GovernanceSeverity::Warning)
            .collect();
        let infos: Vec<&GovernanceViolation> = violations
            .iter()
            .filter(|v| v.severity == GovernanceSeverity::Info)
            .collect();

        if !critical.is_empty() {
            report.push_str(&format!("## CRITICAL ({})\n", critical.len()));
            for v in &critical {
                report.push_str(&format!("- [{}] {}: {}\n", v.rule_id, v.rule_name, v.message));
                report.push_str(&format!("  Recommendation: {}\n", v.recommendation));
            }
            report.push('\n');
        }

        if !errors.is_empty() {
            report.push_str(&format!("## ERRORS ({})\n", errors.len()));
            for v in &errors {
                report.push_str(&format!("- [{}] {}: {}\n", v.rule_id, v.rule_name, v.message));
                report.push_str(&format!("  Recommendation: {}\n", v.recommendation));
            }
            report.push('\n');
        }

        if !warnings.is_empty() {
            report.push_str(&format!("## WARNINGS ({})\n", warnings.len()));
            for v in &warnings {
                report.push_str(&format!("- [{}] {}: {}\n", v.rule_id, v.rule_name, v.message));
            }
            report.push('\n');
        }

        if !infos.is_empty() {
            report.push_str(&format!("## INFO ({})\n", infos.len()));
            for v in &infos {
                report.push_str(&format!("- [{}] {}\n", v.rule_id, v.message));
            }
            report.push('\n');
        }

        report
    }
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Unified Architecture Specification
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Unified enterprise architecture specification combining TOGAF, Zachman, C4, ADRs, and Governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSpec {
    pub project_name: String,
    pub created_at: u64,
    pub togaf: TogafAdm,
    pub zachman: ZachmanFramework,
    pub c4: C4Model,
    pub adrs: AdrStore,
    pub governance: GovernanceEngine,
    pub metadata: HashMap<String, String>,
}

impl ArchitectureSpec {
    pub fn new(project_name: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            created_at: current_timestamp(),
            togaf: TogafAdm::new(),
            zachman: ZachmanFramework::new(),
            c4: C4Model::new(project_name),
            adrs: AdrStore::new(),
            governance: GovernanceEngine::with_standard_rules(),
            metadata: HashMap::new(),
        }
    }

    /// Mutable reference to the TOGAF ADM.
    pub fn togaf(&mut self) -> &mut TogafAdm {
        &mut self.togaf
    }

    /// Mutable reference to the Zachman Framework.
    pub fn zachman(&mut self) -> &mut ZachmanFramework {
        &mut self.zachman
    }

    /// Mutable reference to the C4 Model.
    pub fn c4(&mut self) -> &mut C4Model {
        &mut self.c4
    }

    /// Mutable reference to the ADR Store.
    pub fn adrs(&mut self) -> &mut AdrStore {
        &mut self.adrs
    }

    /// Mutable reference to the Governance Engine.
    pub fn governance(&mut self) -> &mut GovernanceEngine {
        &mut self.governance
    }

    /// Set a metadata key-value pair.
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Generate a comprehensive architecture report.
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!(
            "# Enterprise Architecture Report: {}\n\n",
            self.project_name
        ));

        // TOGAF summary
        report.push_str("## TOGAF ADM Progress\n\n");
        report.push_str(&format!(
            "Overall Progress: {:.0}%\n\n",
            self.togaf.get_overall_progress() * 100.0
        ));
        report.push_str("| Phase | Completion | Artifacts |\n");
        report.push_str("|-------|------------|----------|\n");
        for phase in TogafPhase::all() {
            let completion = self.togaf.get_phase_completion(phase);
            let count = self.togaf.get_artifacts_by_phase(phase).len();
            report.push_str(&format!(
                "| {} | {:.0}% | {} |\n",
                phase.label(),
                completion * 100.0,
                count
            ));
        }
        report.push('\n');

        // Zachman summary
        report.push_str("## Zachman Framework\n\n");
        report.push_str(&format!(
            "Coverage: {:.0}% | Avg Maturity: {:.1}/5\n",
            self.zachman.get_coverage() * 100.0,
            self.zachman.average_maturity()
        ));
        let gaps = self.zachman.get_gaps();
        if !gaps.is_empty() {
            report.push_str(&format!("Gaps: {} cells unfilled\n", gaps.len()));
        }
        report.push('\n');

        // C4 summary
        report.push_str("## C4 Model\n\n");
        report.push_str(&format!("System: {}\n", self.c4.system_name));
        report.push_str(&format!("Elements: {}\n", self.c4.elements.len()));
        report.push_str(&format!("Relationships: {}\n", self.c4.relationships.len()));
        let c4_issues = self.c4.validate_model();
        if !c4_issues.is_empty() {
            report.push_str(&format!("Issues: {}\n", c4_issues.len()));
        }
        report.push('\n');

        // ADR summary
        report.push_str("## Architecture Decisions\n\n");
        report.push_str(&format!("Total ADRs: {}\n", self.adrs.records.len()));
        let summary = self.adrs.status_summary();
        for (status, count) in &summary {
            report.push_str(&format!("- {}: {}\n", status, count));
        }
        report.push('\n');

        // Governance
        let violations = self.governance.evaluate(&self.c4, &self.togaf, &self.adrs);
        report.push_str("## Governance\n\n");
        report.push_str(&format!(
            "Rules: {} | Violations: {}\n",
            self.governance.rules.len(),
            violations.len()
        ));
        let critical_count = violations
            .iter()
            .filter(|v| v.severity == GovernanceSeverity::Critical || v.severity == GovernanceSeverity::Error)
            .count();
        if critical_count > 0 {
            report.push_str(&format!(
                "Critical/Error violations: {}\n",
                critical_count
            ));
        }

        report
    }

    /// Export the entire specification as JSON.
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }

    /// Get a health score (0.0 - 1.0) based on coverage, completion, and governance.
    pub fn health_score(&self) -> f64 {
        let togaf_progress = self.togaf.get_overall_progress();
        let zachman_coverage = self.zachman.get_coverage();
        let c4_issues = self.c4.validate_model().len();
        let c4_health = if self.c4.elements.is_empty() {
            0.0
        } else {
            let issue_ratio = c4_issues as f64 / self.c4.elements.len() as f64;
            (1.0 - issue_ratio).max(0.0)
        };
        let violations = self.governance.evaluate(&self.c4, &self.togaf, &self.adrs);
        let gov_health = if self.governance.rules.is_empty() {
            1.0
        } else {
            let violation_ratio = violations.len() as f64 / self.governance.rules.len() as f64;
            (1.0 - violation_ratio).max(0.0)
        };

        // Weighted average
        (togaf_progress * 0.25 + zachman_coverage * 0.20 + c4_health * 0.30 + gov_health * 0.25)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn generate_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}-{:06}", prefix, counter)
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace('-', "_")
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── TOGAF Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_togaf_phase_all() {
        assert_eq!(TogafPhase::all().len(), 9);
    }

    #[test]
    fn test_togaf_phase_labels() {
        assert_eq!(TogafPhase::Preliminary.label(), "Preliminary");
        assert_eq!(
            TogafPhase::ArchitectureVision.label(),
            "A: Architecture Vision"
        );
        assert_eq!(
            TogafPhase::ArchitectureChangeManagement.label(),
            "H: Architecture Change Management"
        );
    }

    #[test]
    fn test_togaf_phase_order() {
        assert_eq!(TogafPhase::Preliminary.order(), 0);
        assert_eq!(TogafPhase::ArchitectureChangeManagement.order(), 8);
    }

    #[test]
    fn test_togaf_phase_display() {
        let phase = TogafPhase::BusinessArchitecture;
        assert_eq!(format!("{}", phase), "B: Business Architecture");
    }

    #[test]
    fn test_togaf_adm_new() {
        let adm = TogafAdm::new();
        assert!(adm.artifacts.is_empty());
        assert!(adm.principles.is_empty());
    }

    #[test]
    fn test_togaf_add_artifact() {
        let mut adm = TogafAdm::new();
        let art = TogafArtifact::new(
            "Architecture Principles",
            TogafPhase::Preliminary,
            ArtifactType::Catalog,
            "Core architecture principles",
        );
        let id = adm.add_artifact(art);
        assert!(!id.is_empty());
        assert_eq!(adm.artifacts.len(), 1);
    }

    #[test]
    fn test_togaf_get_artifacts_by_phase() {
        let mut adm = TogafAdm::new();
        adm.add_artifact(TogafArtifact::new(
            "Principles",
            TogafPhase::Preliminary,
            ArtifactType::Catalog,
            "Principles catalog",
        ));
        adm.add_artifact(TogafArtifact::new(
            "Vision",
            TogafPhase::ArchitectureVision,
            ArtifactType::Diagram,
            "Vision diagram",
        ));
        adm.add_artifact(TogafArtifact::new(
            "Stakeholder Map",
            TogafPhase::Preliminary,
            ArtifactType::Matrix,
            "Stakeholders",
        ));

        let prelim = adm.get_artifacts_by_phase(&TogafPhase::Preliminary);
        assert_eq!(prelim.len(), 2);
        let vision = adm.get_artifacts_by_phase(&TogafPhase::ArchitectureVision);
        assert_eq!(vision.len(), 1);
    }

    #[test]
    fn test_togaf_phase_completion_empty() {
        let adm = TogafAdm::new();
        assert_eq!(adm.get_phase_completion(&TogafPhase::Preliminary), 0.0);
    }

    #[test]
    fn test_togaf_phase_completion_partial() {
        let mut adm = TogafAdm::new();
        // Preliminary requires "Architecture Principles" and "Stakeholder Map"
        adm.add_artifact(
            TogafArtifact::new(
                "Architecture Principles",
                TogafPhase::Preliminary,
                ArtifactType::Catalog,
                "Principles",
            )
            .with_status(ArtifactStatus::Approved),
        );
        let completion = adm.get_phase_completion(&TogafPhase::Preliminary);
        assert!(completion > 0.0 && completion < 1.0);
    }

    #[test]
    fn test_togaf_phase_completion_full() {
        let mut adm = TogafAdm::new();
        adm.add_artifact(
            TogafArtifact::new(
                "Architecture Principles",
                TogafPhase::Preliminary,
                ArtifactType::Catalog,
                "Principles",
            )
            .with_status(ArtifactStatus::Approved),
        );
        adm.add_artifact(
            TogafArtifact::new(
                "Stakeholder Map",
                TogafPhase::Preliminary,
                ArtifactType::Matrix,
                "Stakeholders",
            )
            .with_status(ArtifactStatus::Approved),
        );
        let completion = adm.get_phase_completion(&TogafPhase::Preliminary);
        assert_eq!(completion, 1.0);
    }

    #[test]
    fn test_togaf_overall_progress() {
        let adm = TogafAdm::new();
        assert_eq!(adm.get_overall_progress(), 0.0);
    }

    #[test]
    fn test_togaf_validate_prerequisites_missing() {
        let adm = TogafAdm::new();
        let missing = adm.validate_phase_prerequisites(&TogafPhase::Preliminary);
        assert_eq!(missing.len(), 2); // Principles + Stakeholder Map
    }

    #[test]
    fn test_togaf_validate_prerequisites_satisfied() {
        let mut adm = TogafAdm::new();
        adm.add_artifact(TogafArtifact::new(
            "Architecture Principles",
            TogafPhase::Preliminary,
            ArtifactType::Catalog,
            "Core principles",
        ));
        adm.add_artifact(TogafArtifact::new(
            "Stakeholder Map",
            TogafPhase::Preliminary,
            ArtifactType::Matrix,
            "Stakeholder mapping",
        ));
        let missing = adm.validate_phase_prerequisites(&TogafPhase::Preliminary);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_togaf_generate_phase_report() {
        let mut adm = TogafAdm::new();
        adm.add_artifact(TogafArtifact::new(
            "Vision Document",
            TogafPhase::ArchitectureVision,
            ArtifactType::Catalog,
            "Architecture vision",
        ));
        let report = adm.generate_phase_report(&TogafPhase::ArchitectureVision);
        assert!(report.contains("Architecture Vision"));
        assert!(report.contains("Vision Document"));
    }

    #[test]
    fn test_togaf_update_artifact_status() {
        let mut adm = TogafAdm::new();
        let id = adm.add_artifact(TogafArtifact::new(
            "Test",
            TogafPhase::Preliminary,
            ArtifactType::Catalog,
            "Test artifact",
        ));
        assert!(adm.update_artifact_status(&id, ArtifactStatus::Approved));
        let art = adm.artifacts.iter().find(|a| a.id == id).unwrap();
        assert_eq!(art.status, ArtifactStatus::Approved);
    }

    #[test]
    fn test_togaf_update_artifact_status_not_found() {
        let mut adm = TogafAdm::new();
        assert!(!adm.update_artifact_status("nonexistent", ArtifactStatus::Approved));
    }

    #[test]
    fn test_togaf_get_artifacts_by_status() {
        let mut adm = TogafAdm::new();
        adm.add_artifact(
            TogafArtifact::new("A", TogafPhase::Preliminary, ArtifactType::Catalog, "a")
                .with_status(ArtifactStatus::Approved),
        );
        adm.add_artifact(
            TogafArtifact::new("B", TogafPhase::Preliminary, ArtifactType::Catalog, "b")
                .with_status(ArtifactStatus::Draft),
        );
        assert_eq!(adm.get_artifacts_by_status(&ArtifactStatus::Approved).len(), 1);
        assert_eq!(adm.get_artifacts_by_status(&ArtifactStatus::Draft).len(), 1);
    }

    #[test]
    fn test_togaf_artifact_with_content() {
        let art = TogafArtifact::new("Test", TogafPhase::Preliminary, ArtifactType::Catalog, "desc")
            .with_content("Some content here");
        assert_eq!(art.content, "Some content here");
    }

    #[test]
    fn test_togaf_artifact_with_tags() {
        let art = TogafArtifact::new("Test", TogafPhase::Preliminary, ArtifactType::Catalog, "desc")
            .with_tags(vec!["security".to_string(), "cloud".to_string()]);
        assert_eq!(art.tags.len(), 2);
    }

    #[test]
    fn test_togaf_add_principle() {
        let mut adm = TogafAdm::new();
        let principle = ArchitecturePrinciple {
            id: "AP-001".to_string(),
            name: "Technology Independence".to_string(),
            statement: "Architecture should not depend on specific products".to_string(),
            rationale: "Reduces vendor lock-in".to_string(),
            implications: vec!["Use abstraction layers".to_string()],
        };
        let id = adm.add_principle(principle);
        assert_eq!(id, "AP-001");
        assert_eq!(adm.principles.len(), 1);
    }

    // ── Zachman Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_zachman_perspective_all() {
        assert_eq!(ZachmanPerspective::all().len(), 6);
    }

    #[test]
    fn test_zachman_aspect_all() {
        assert_eq!(ZachmanAspect::all().len(), 6);
    }

    #[test]
    fn test_zachman_new_empty() {
        let zf = ZachmanFramework::new();
        assert_eq!(zf.get_coverage(), 0.0);
    }

    #[test]
    fn test_zachman_set_and_get_cell() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(
            ZachmanPerspective::Planner,
            ZachmanAspect::What,
            "Business entities list",
        );
        let cell = zf
            .get_cell(&ZachmanPerspective::Planner, &ZachmanAspect::What)
            .unwrap();
        assert_eq!(cell.content, "Business entities list");
        assert_eq!(cell.maturity, 1);
    }

    #[test]
    fn test_zachman_coverage_one_cell() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(
            ZachmanPerspective::Planner,
            ZachmanAspect::What,
            "Entities",
        );
        let coverage = zf.get_coverage();
        assert!((coverage - 1.0 / 36.0).abs() < 0.001);
    }

    #[test]
    fn test_zachman_get_gaps() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Planner, ZachmanAspect::What, "X");
        let gaps = zf.get_gaps();
        assert_eq!(gaps.len(), 35); // 36 - 1
    }

    #[test]
    fn test_zachman_all_cells_filled() {
        let mut zf = ZachmanFramework::new();
        for p in ZachmanPerspective::all() {
            for a in ZachmanAspect::all() {
                zf.set_cell(p.clone(), a.clone(), "filled");
            }
        }
        assert_eq!(zf.get_coverage(), 1.0);
        assert!(zf.get_gaps().is_empty());
    }

    #[test]
    fn test_zachman_cell_maturity() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Owner, ZachmanAspect::How, "Processes");
        zf.set_cell_maturity(&ZachmanPerspective::Owner, &ZachmanAspect::How, 4);
        let cell = zf.get_cell(&ZachmanPerspective::Owner, &ZachmanAspect::How).unwrap();
        assert_eq!(cell.maturity, 4);
    }

    #[test]
    fn test_zachman_maturity_capped_at_5() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Owner, ZachmanAspect::How, "X");
        zf.set_cell_maturity(&ZachmanPerspective::Owner, &ZachmanAspect::How, 10);
        let cell = zf.get_cell(&ZachmanPerspective::Owner, &ZachmanAspect::How).unwrap();
        assert_eq!(cell.maturity, 5);
    }

    #[test]
    fn test_zachman_average_maturity() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Planner, ZachmanAspect::What, "A");
        zf.set_cell_maturity(&ZachmanPerspective::Planner, &ZachmanAspect::What, 4);
        zf.set_cell(ZachmanPerspective::Owner, ZachmanAspect::How, "B");
        zf.set_cell_maturity(&ZachmanPerspective::Owner, &ZachmanAspect::How, 2);
        assert!((zf.average_maturity() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_zachman_add_cell_artifact() {
        let mut zf = ZachmanFramework::new();
        zf.add_cell_artifact(
            &ZachmanPerspective::Designer,
            &ZachmanAspect::What,
            "ERD-001",
        );
        let cell = zf.get_cell(&ZachmanPerspective::Designer, &ZachmanAspect::What).unwrap();
        assert_eq!(cell.artifacts.len(), 1);
        assert_eq!(cell.artifacts[0], "ERD-001");
    }

    #[test]
    fn test_zachman_generate_matrix_report() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Planner, ZachmanAspect::What, "Entities");
        let report = zf.generate_matrix_report();
        assert!(report.contains("Zachman Framework Matrix"));
        assert!(report.contains("Coverage:"));
    }

    #[test]
    fn test_zachman_validate_consistency_clean() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Owner, ZachmanAspect::What, "Business model");
        zf.set_cell(ZachmanPerspective::Designer, ZachmanAspect::What, "Logical model");
        let issues = zf.validate_consistency();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_zachman_validate_consistency_missing_owner() {
        let mut zf = ZachmanFramework::new();
        // Designer has content but Owner does not
        zf.set_cell(ZachmanPerspective::Designer, ZachmanAspect::What, "Logical model");
        let issues = zf.validate_consistency();
        assert!(issues.iter().any(|i| i.contains("Owner row is empty")));
    }

    #[test]
    fn test_zachman_validate_consistency_missing_designer() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Builder, ZachmanAspect::How, "Physical process");
        let issues = zf.validate_consistency();
        assert!(issues.iter().any(|i| i.contains("Designer row is empty")));
    }

    #[test]
    fn test_zachman_validate_planner_requires_what_why() {
        let mut zf = ZachmanFramework::new();
        zf.set_cell(ZachmanPerspective::Planner, ZachmanAspect::How, "Process flow");
        let issues = zf.validate_consistency();
        assert!(issues.iter().any(|i| i.contains("What (Data) is missing")));
        assert!(issues.iter().any(|i| i.contains("Why (Motivation) is missing")));
    }

    #[test]
    fn test_zachman_cell_empty_check() {
        let cell = ZachmanCell::new(ZachmanPerspective::Worker, ZachmanAspect::When);
        assert!(cell.is_empty());
    }

    #[test]
    fn test_zachman_perspective_display() {
        assert_eq!(
            format!("{}", ZachmanPerspective::Planner),
            "Planner (Scope)"
        );
    }

    #[test]
    fn test_zachman_aspect_display() {
        assert_eq!(format!("{}", ZachmanAspect::What), "What (Data)");
    }

    // ── C4 Model Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_c4_model_new() {
        let model = C4Model::new("Test System");
        assert_eq!(model.system_name, "Test System");
        assert!(model.elements.is_empty());
        assert!(model.relationships.is_empty());
    }

    #[test]
    fn test_c4_add_element() {
        let mut model = C4Model::new("Test");
        let el = C4Element::new("User", C4ElementType::Person, "End user");
        let id = model.add_element(el);
        assert!(!id.is_empty());
        assert_eq!(model.elements.len(), 1);
    }

    #[test]
    fn test_c4_add_relationship() {
        let mut model = C4Model::new("Test");
        let user_id = model.add_element(C4Element::new("User", C4ElementType::Person, "User"));
        let sys_id = model.add_element(C4Element::new(
            "System",
            C4ElementType::SoftwareSystem,
            "Main system",
        ));
        model.add_relationship(C4Relationship::new(&user_id, &sys_id, "Uses"));
        assert_eq!(model.relationships.len(), 1);
    }

    #[test]
    fn test_c4_get_element() {
        let mut model = C4Model::new("Test");
        let id = model.add_element(C4Element::new("API", C4ElementType::Container, "REST API"));
        assert!(model.get_element(&id).is_some());
        assert!(model.get_element("nonexistent").is_none());
    }

    #[test]
    fn test_c4_get_elements_by_type() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("User", C4ElementType::Person, "User"));
        model.add_element(C4Element::new("Admin", C4ElementType::Person, "Admin"));
        model.add_element(C4Element::new("System", C4ElementType::SoftwareSystem, "Sys"));
        assert_eq!(model.get_elements_by_type(&C4ElementType::Person).len(), 2);
        assert_eq!(
            model
                .get_elements_by_type(&C4ElementType::SoftwareSystem)
                .len(),
            1
        );
    }

    #[test]
    fn test_c4_get_children() {
        let mut model = C4Model::new("Test");
        let sys_id = model.add_element(C4Element::new(
            "System",
            C4ElementType::SoftwareSystem,
            "Main",
        ));
        model.add_element(
            C4Element::new("API", C4ElementType::Container, "REST API")
                .with_parent(&sys_id),
        );
        model.add_element(
            C4Element::new("DB", C4ElementType::Container, "Database")
                .with_parent(&sys_id),
        );
        assert_eq!(model.get_children(&sys_id).len(), 2);
    }

    #[test]
    fn test_c4_get_relationships_for() {
        let mut model = C4Model::new("Test");
        let a = model.add_element(C4Element::new("A", C4ElementType::Container, "A"));
        let b = model.add_element(C4Element::new("B", C4ElementType::Container, "B"));
        let c = model.add_element(C4Element::new("C", C4ElementType::Container, "C"));
        model.add_relationship(C4Relationship::new(&a, &b, "calls"));
        model.add_relationship(C4Relationship::new(&b, &c, "reads"));
        assert_eq!(model.get_relationships_for(&b).len(), 2);
        assert_eq!(model.get_relationships_for(&a).len(), 1);
    }

    #[test]
    fn test_c4_generate_context_diagram() {
        let mut model = C4Model::new("E-Commerce");
        let user_id = model.add_element(
            C4Element::new("Customer", C4ElementType::Person, "Online shopper")
                .with_id("user1"),
        );
        let sys_id = model.add_element(
            C4Element::new("E-Commerce Platform", C4ElementType::SoftwareSystem, "Main platform")
                .with_id("sys1"),
        );
        model.add_relationship(
            C4Relationship::new(&user_id, &sys_id, "Browses and purchases").with_technology("HTTPS"),
        );
        let diagram = model.generate_context_diagram();
        assert!(diagram.contains("graph TB"));
        assert!(diagram.contains("Customer"));
        assert!(diagram.contains("E-Commerce Platform"));
    }

    #[test]
    fn test_c4_generate_container_diagram() {
        let mut model = C4Model::new("Test");
        model.add_element(
            C4Element::new("Web App", C4ElementType::Container, "Frontend")
                .with_technology("React")
                .with_id("webapp"),
        );
        let diagram = model.generate_container_diagram();
        assert!(diagram.contains("Container Diagram"));
        assert!(diagram.contains("Web App"));
    }

    #[test]
    fn test_c4_generate_component_diagram() {
        let mut model = C4Model::new("Test");
        let container_id = model.add_element(
            C4Element::new("API", C4ElementType::Container, "REST API").with_id("api1"),
        );
        model.add_element(
            C4Element::new("AuthController", C4ElementType::Component, "Handles auth")
                .with_technology("Rust")
                .with_parent(&container_id)
                .with_id("auth1"),
        );
        model.add_element(
            C4Element::new("UserController", C4ElementType::Component, "Handles users")
                .with_parent(&container_id)
                .with_id("user_ctrl"),
        );
        let diagram = model.generate_component_diagram(&container_id);
        assert!(diagram.contains("Component Diagram"));
        assert!(diagram.contains("AuthController"));
    }

    #[test]
    fn test_c4_validate_orphan_elements() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("Orphan", C4ElementType::SoftwareSystem, "No rels"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("Orphan")));
    }

    #[test]
    fn test_c4_validate_dangling_refs() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("A", C4ElementType::Container, "A").with_id("a1"));
        model.add_relationship(C4Relationship::new("a1", "nonexistent", "calls"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("unknown target")));
    }

    #[test]
    fn test_c4_validate_self_reference() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("A", C4ElementType::Container, "A").with_id("a1"));
        model.add_relationship(C4Relationship::new("a1", "a1", "recursive"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("Self-referencing")));
    }

    #[test]
    fn test_c4_validate_container_no_parent() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("API", C4ElementType::Container, "No parent"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("no parent system")));
    }

    #[test]
    fn test_c4_validate_component_no_parent() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("Ctrl", C4ElementType::Component, "No parent"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("no parent container")));
    }

    #[test]
    fn test_c4_validate_missing_description() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("X", C4ElementType::SoftwareSystem, ""));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("no description")));
    }

    #[test]
    fn test_c4_validate_duplicate_names() {
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("API", C4ElementType::Container, "first"));
        model.add_element(C4Element::new("API", C4ElementType::Container, "second"));
        let issues = model.validate_model();
        assert!(issues.iter().any(|i| i.contains("Duplicate")));
    }

    #[test]
    fn test_c4_element_with_technology() {
        let el = C4Element::new("API", C4ElementType::Container, "API")
            .with_technology("Rust/Actix");
        assert_eq!(el.technology, "Rust/Actix");
    }

    #[test]
    fn test_c4_relationship_with_technology() {
        let rel = C4Relationship::new("a", "b", "calls").with_technology("gRPC");
        assert_eq!(rel.technology, "gRPC");
    }

    #[test]
    fn test_c4_level_label() {
        assert_eq!(C4Level::Context.label(), "System Context");
        assert_eq!(C4Level::Code.label(), "Code");
    }

    // ── ADR Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_adr_store_new() {
        let store = AdrStore::new();
        assert!(store.records.is_empty());
    }

    #[test]
    fn test_adr_add_and_get() {
        let mut store = AdrStore::new();
        let adr = Adr::new("Use Rust", "Need performance", "Adopt Rust for backend");
        let id = store.add(adr);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn test_adr_list() {
        let mut store = AdrStore::new();
        store.add(Adr::new("ADR 1", "ctx1", "dec1"));
        store.add(Adr::new("ADR 2", "ctx2", "dec2"));
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn test_adr_accept() {
        let mut store = AdrStore::new();
        let id = store.add(Adr::new("Test", "ctx", "dec"));
        assert!(store.accept(&id));
        assert_eq!(store.get(&id).unwrap().status, AdrStatus::Accepted);
    }

    #[test]
    fn test_adr_deprecate() {
        let mut store = AdrStore::new();
        let id = store.add(Adr::new("Test", "ctx", "dec"));
        assert!(store.deprecate(&id));
        assert_eq!(store.get(&id).unwrap().status, AdrStatus::Deprecated);
    }

    #[test]
    fn test_adr_supersede() {
        let mut store = AdrStore::new();
        let old_id = store.add(Adr::new("Old", "ctx", "dec"));
        let new_id = store.add(Adr::new("New", "ctx", "dec"));
        assert!(store.supersede(&old_id, &new_id));
        assert_eq!(
            store.get(&old_id).unwrap().status,
            AdrStatus::Superseded(new_id)
        );
    }

    #[test]
    fn test_adr_accept_nonexistent() {
        let mut store = AdrStore::new();
        assert!(!store.accept("fake-id"));
    }

    #[test]
    fn test_adr_generate_markdown() {
        let mut store = AdrStore::new();
        let adr = Adr::new("Use PostgreSQL", "Need relational DB", "Adopt PostgreSQL")
            .with_consequences(vec!["Need DBA expertise".to_string()])
            .with_participants(vec!["Alice".to_string(), "Bob".to_string()]);
        let id = store.add(adr);
        let md = store.generate_markdown(&id);
        assert!(md.contains("Use PostgreSQL"));
        assert!(md.contains("Need relational DB"));
        assert!(md.contains("Adopt PostgreSQL"));
        assert!(md.contains("Need DBA expertise"));
        assert!(md.contains("Alice, Bob"));
    }

    #[test]
    fn test_adr_generate_markdown_not_found() {
        let store = AdrStore::new();
        let md = store.generate_markdown("nope");
        assert!(md.contains("not found"));
    }

    #[test]
    fn test_adr_generate_index() {
        let mut store = AdrStore::new();
        store.add(Adr::new("ADR 1", "ctx", "dec"));
        store.add(Adr::new("ADR 2", "ctx", "dec"));
        let index = store.generate_index();
        assert!(index.contains("ADR 1"));
        assert!(index.contains("ADR 2"));
        assert!(index.contains("Total: 2"));
    }

    #[test]
    fn test_adr_search_by_title() {
        let mut store = AdrStore::new();
        store.add(Adr::new("Use Kubernetes", "Need orchestration", "Adopt K8s"));
        store.add(Adr::new("Use Docker", "Need containers", "Adopt Docker"));
        let results = store.search("kubernetes");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Use Kubernetes");
    }

    #[test]
    fn test_adr_search_by_context() {
        let mut store = AdrStore::new();
        store.add(Adr::new("Use X", "Performance is critical", "Adopt X"));
        let results = store.search("performance");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_adr_search_by_tag() {
        let mut store = AdrStore::new();
        store.add(
            Adr::new("Use X", "ctx", "dec").with_tags(vec!["infrastructure".to_string()]),
        );
        let results = store.search("infrastructure");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_adr_search_no_results() {
        let mut store = AdrStore::new();
        store.add(Adr::new("Use X", "ctx", "dec"));
        let results = store.search("zzznotfound");
        assert!(results.is_empty());
    }

    #[test]
    fn test_adr_get_by_status() {
        let mut store = AdrStore::new();
        let id1 = store.add(Adr::new("A", "c", "d"));
        let _id2 = store.add(Adr::new("B", "c", "d"));
        store.accept(&id1);
        assert_eq!(store.get_by_status(&AdrStatus::Accepted).len(), 1);
        assert_eq!(store.get_by_status(&AdrStatus::Proposed).len(), 1);
    }

    #[test]
    fn test_adr_status_summary() {
        let mut store = AdrStore::new();
        let id1 = store.add(Adr::new("A", "c", "d"));
        store.add(Adr::new("B", "c", "d"));
        store.accept(&id1);
        let summary = store.status_summary();
        assert_eq!(summary.get("Accepted"), Some(&1));
        assert_eq!(summary.get("Proposed"), Some(&1));
    }

    #[test]
    fn test_adr_status_display() {
        assert_eq!(format!("{}", AdrStatus::Proposed), "Proposed");
        assert_eq!(
            format!("{}", AdrStatus::Superseded("ADR-002".to_string())),
            "Superseded by ADR-002"
        );
    }

    #[test]
    fn test_adr_with_date() {
        let adr = Adr::new("Test", "ctx", "dec").with_date("2026-01-15");
        assert_eq!(adr.date, "2026-01-15");
    }

    // ── Governance Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_governance_engine_new() {
        let engine = GovernanceEngine::new();
        assert!(engine.rules.is_empty());
    }

    #[test]
    fn test_governance_with_standard_rules() {
        let engine = GovernanceEngine::with_standard_rules();
        assert_eq!(engine.rules.len(), 7);
    }

    #[test]
    fn test_governance_add_rule() {
        let mut engine = GovernanceEngine::new();
        engine.add_rule(GovernanceRule::new(
            "Test Rule",
            "A test",
            GovernanceSeverity::Warning,
        ));
        assert_eq!(engine.rules.len(), 1);
    }

    #[test]
    fn test_governance_get_rule() {
        let engine = GovernanceEngine::with_standard_rules();
        assert!(engine.get_rule("GOV-001").is_some());
        assert!(engine.get_rule("GOV-999").is_none());
    }

    #[test]
    fn test_governance_evaluate_c4_issues() {
        let engine = GovernanceEngine::with_standard_rules();
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("Orphan", C4ElementType::SoftwareSystem, ""));
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-001"));
    }

    #[test]
    fn test_governance_evaluate_togaf_prerequisites() {
        let engine = GovernanceEngine::with_standard_rules();
        let model = C4Model::new("Test");
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-002"));
    }

    #[test]
    fn test_governance_evaluate_adr_coverage() {
        let engine = GovernanceEngine::with_standard_rules();
        let model = C4Model::new("Test");
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-003"));
    }

    #[test]
    fn test_governance_evaluate_container_technology() {
        let engine = GovernanceEngine::with_standard_rules();
        let mut model = C4Model::new("Test");
        model.add_element(
            C4Element::new("API", C4ElementType::Container, "API")
                .with_parent("sys1"),
        );
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-004"));
    }

    #[test]
    fn test_governance_evaluate_draft_artifacts() {
        let engine = GovernanceEngine::with_standard_rules();
        let model = C4Model::new("Test");
        let mut adm = TogafAdm::new();
        // Add 3 draft artifacts to Preliminary
        adm.add_artifact(TogafArtifact::new(
            "A", TogafPhase::Preliminary, ArtifactType::Catalog, "a",
        ));
        adm.add_artifact(TogafArtifact::new(
            "B", TogafPhase::Preliminary, ArtifactType::Matrix, "b",
        ));
        adm.add_artifact(TogafArtifact::new(
            "C", TogafPhase::Preliminary, ArtifactType::Diagram, "c",
        ));
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-005"));
    }

    #[test]
    fn test_governance_evaluate_container_parentage() {
        let engine = GovernanceEngine::with_standard_rules();
        let mut model = C4Model::new("Test");
        model.add_element(C4Element::new("API", C4ElementType::Container, "No parent"));
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-006"));
    }

    #[test]
    fn test_governance_evaluate_supersession_chain() {
        let engine = GovernanceEngine::with_standard_rules();
        let model = C4Model::new("Test");
        let adm = TogafAdm::new();
        let mut adrs = AdrStore::new();
        let old_id = adrs.add(Adr::new("Old", "c", "d"));
        adrs.supersede(&old_id, "nonexistent-adr");
        let violations = engine.evaluate(&model, &adm, &adrs);
        assert!(violations.iter().any(|v| v.rule_id == "GOV-007"));
    }

    #[test]
    fn test_governance_generate_report() {
        let engine = GovernanceEngine::with_standard_rules();
        let model = C4Model::new("Test");
        let adm = TogafAdm::new();
        let adrs = AdrStore::new();
        let report = engine.generate_report(&model, &adm, &adrs);
        assert!(report.contains("Architecture Governance Report"));
        assert!(report.contains("Violations:"));
    }

    #[test]
    fn test_governance_severity_display() {
        assert_eq!(format!("{}", GovernanceSeverity::Critical), "CRITICAL");
        assert_eq!(format!("{}", GovernanceSeverity::Info), "INFO");
    }

    #[test]
    fn test_governance_rule_builder() {
        let rule = GovernanceRule::new("Test", "Description", GovernanceSeverity::Error)
            .with_check_description("Checks something")
            .with_category("security")
            .with_id("CUSTOM-001");
        assert_eq!(rule.id, "CUSTOM-001");
        assert_eq!(rule.category, "security");
        assert_eq!(rule.check_fn_description, "Checks something");
    }

    // ── Unified ArchitectureSpec Tests ───────────────────────────────────────

    #[test]
    fn test_architecture_spec_new() {
        let spec = ArchitectureSpec::new("My Project");
        assert_eq!(spec.project_name, "My Project");
        assert_eq!(spec.c4.system_name, "My Project");
    }

    #[test]
    fn test_architecture_spec_togaf_mut() {
        let mut spec = ArchitectureSpec::new("Test");
        spec.togaf().add_artifact(TogafArtifact::new(
            "Principles",
            TogafPhase::Preliminary,
            ArtifactType::Catalog,
            "desc",
        ));
        assert_eq!(spec.togaf.artifacts.len(), 1);
    }

    #[test]
    fn test_architecture_spec_zachman_mut() {
        let mut spec = ArchitectureSpec::new("Test");
        spec.zachman()
            .set_cell(ZachmanPerspective::Planner, ZachmanAspect::What, "Entities");
        assert!(spec.zachman.get_coverage() > 0.0);
    }

    #[test]
    fn test_architecture_spec_c4_mut() {
        let mut spec = ArchitectureSpec::new("Test");
        spec.c4()
            .add_element(C4Element::new("User", C4ElementType::Person, "End user"));
        assert_eq!(spec.c4.elements.len(), 1);
    }

    #[test]
    fn test_architecture_spec_adrs_mut() {
        let mut spec = ArchitectureSpec::new("Test");
        spec.adrs().add(Adr::new("ADR 1", "ctx", "dec"));
        assert_eq!(spec.adrs.records.len(), 1);
    }

    #[test]
    fn test_architecture_spec_governance_mut() {
        let mut spec = ArchitectureSpec::new("Test");
        let initial_count = spec.governance.rules.len();
        spec.governance().add_rule(GovernanceRule::new(
            "Custom",
            "desc",
            GovernanceSeverity::Info,
        ));
        assert_eq!(spec.governance.rules.len(), initial_count + 1);
    }

    #[test]
    fn test_architecture_spec_generate_report() {
        let spec = ArchitectureSpec::new("Acme Platform");
        let report = spec.generate_report();
        assert!(report.contains("Acme Platform"));
        assert!(report.contains("TOGAF ADM Progress"));
        assert!(report.contains("Zachman Framework"));
        assert!(report.contains("C4 Model"));
        assert!(report.contains("Architecture Decisions"));
        assert!(report.contains("Governance"));
    }

    #[test]
    fn test_architecture_spec_export_json() {
        let spec = ArchitectureSpec::new("Test");
        let json = spec.export_json();
        assert!(json.contains("project_name"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_architecture_spec_health_score_empty() {
        let spec = ArchitectureSpec::new("Test");
        let score = spec.health_score();
        // Empty spec should have low health (governance violations exist)
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_architecture_spec_set_metadata() {
        let mut spec = ArchitectureSpec::new("Test");
        spec.set_metadata("version", "1.0");
        assert_eq!(spec.metadata.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_architecture_spec_full_lifecycle() {
        let mut spec = ArchitectureSpec::new("E-Commerce Platform");

        // Add TOGAF artifacts
        spec.togaf().add_artifact(
            TogafArtifact::new(
                "Architecture Principles",
                TogafPhase::Preliminary,
                ArtifactType::Catalog,
                "Core principles",
            )
            .with_status(ArtifactStatus::Approved),
        );
        spec.togaf().add_artifact(
            TogafArtifact::new(
                "Stakeholder Map",
                TogafPhase::Preliminary,
                ArtifactType::Matrix,
                "Key stakeholders",
            )
            .with_status(ArtifactStatus::Approved),
        );

        // Fill Zachman cells
        spec.zachman().set_cell(
            ZachmanPerspective::Planner,
            ZachmanAspect::What,
            "Products, Orders, Customers",
        );
        spec.zachman().set_cell(
            ZachmanPerspective::Planner,
            ZachmanAspect::Why,
            "Enable online commerce",
        );

        // Build C4 model
        let user_id = spec.c4().add_element(
            C4Element::new("Customer", C4ElementType::Person, "Online shopper")
                .with_id("customer"),
        );
        let sys_id = spec.c4().add_element(
            C4Element::new("E-Commerce", C4ElementType::SoftwareSystem, "Main platform")
                .with_id("ecommerce"),
        );
        spec.c4().add_relationship(C4Relationship::new(&user_id, &sys_id, "Shops"));

        // Add ADRs
        let adr_id = spec.adrs().add(Adr::new(
            "Use Microservices",
            "Need independent deployability",
            "Adopt microservices architecture",
        ));
        spec.adrs().accept(&adr_id);

        // Verify
        assert_eq!(
            spec.togaf().get_phase_completion(&TogafPhase::Preliminary),
            1.0
        );
        assert!(spec.zachman().get_coverage() > 0.0);
        assert_eq!(spec.c4.elements.len(), 2);
        assert_eq!(spec.adrs.records.len(), 1);

        let report = spec.generate_report();
        assert!(report.contains("E-Commerce Platform"));
        assert!(report.contains("100%")); // Preliminary phase

        let json = spec.export_json();
        assert!(json.contains("E-Commerce Platform"));
        assert!(json.contains("Microservices"));
    }

    // ── Helper Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_generate_id_unique() {
        let id1 = generate_id("test");
        let id2 = generate_id("test");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_id_prefix() {
        let id = generate_id("prefix");
        assert!(id.starts_with("prefix-"));
    }

    #[test]
    fn test_sanitize_mermaid_id() {
        assert_eq!(sanitize_mermaid_id("c4-001"), "c4_001");
        assert_eq!(sanitize_mermaid_id("no-dashes-here"), "no_dashes_here");
    }

    #[test]
    fn test_artifact_type_display() {
        assert_eq!(format!("{}", ArtifactType::Catalog), "Catalog");
        assert_eq!(format!("{}", ArtifactType::Matrix), "Matrix");
        assert_eq!(format!("{}", ArtifactType::Diagram), "Diagram");
    }

    #[test]
    fn test_artifact_status_display() {
        assert_eq!(format!("{}", ArtifactStatus::Draft), "Draft");
        assert_eq!(format!("{}", ArtifactStatus::Approved), "Approved");
    }

    #[test]
    fn test_c4_element_type_display() {
        assert_eq!(format!("{}", C4ElementType::Person), "Person");
        assert_eq!(format!("{}", C4ElementType::SoftwareSystem), "Software System");
    }
}
