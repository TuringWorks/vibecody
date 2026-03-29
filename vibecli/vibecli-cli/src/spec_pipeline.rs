#![allow(dead_code)]
//! Spec-driven development pipeline for VibeCody.
//!
//! Implements a structured requirements → design → tasks pipeline with EARS
//! (Easy Approach to Requirements Syntax) parsing. Inspired by AWS Kiro's
//! spec-driven approach.
//!
//! REPL commands: `/spec init|req|design|task|link|validate|summary`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum EarsPattern {
    /// "The [system] shall [action]"
    Ubiquitous,
    /// "When [trigger], the [system] shall [action]"
    EventDriven,
    /// "If [condition], then the [system] shall [action]"
    UnwantedBehavior,
    /// "While [state], the [system] shall [action]"
    StateDriven,
    /// "Where [feature], the [system] shall [action]"
    Optional,
}

impl std::fmt::Display for EarsPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ubiquitous => write!(f, "ubiquitous"),
            Self::EventDriven => write!(f, "event-driven"),
            Self::UnwantedBehavior => write!(f, "unwanted-behavior"),
            Self::StateDriven => write!(f, "state-driven"),
            Self::Optional => write!(f, "optional"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Priority {
    Must,
    Should,
    Could,
    WontHave,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Must => write!(f, "Must"),
            Self::Should => write!(f, "Should"),
            Self::Could => write!(f, "Could"),
            Self::WontHave => write!(f, "Won't Have"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequirementStatus {
    Draft,
    Approved,
    Implemented,
    Verified,
}

impl std::fmt::Display for RequirementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Approved => write!(f, "approved"),
            Self::Implemented => write!(f, "implemented"),
            Self::Verified => write!(f, "verified"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DesignStatus {
    Proposed,
    Approved,
    Implemented,
}

impl std::fmt::Display for DesignStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "proposed"),
            Self::Approved => write!(f, "approved"),
            Self::Implemented => write!(f, "implemented"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Todo => write!(f, "todo"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Done => write!(f, "done"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effort {
    Small,
    Medium,
    Large,
    XLarge,
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Small => write!(f, "S"),
            Self::Medium => write!(f, "M"),
            Self::Large => write!(f, "L"),
            Self::XLarge => write!(f, "XL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceType {
    Api,
    Event,
    DataStore,
    FileSystem,
    Network,
}

impl std::fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api => write!(f, "API"),
            Self::Event => write!(f, "Event"),
            Self::DataStore => write!(f, "DataStore"),
            Self::FileSystem => write!(f, "FileSystem"),
            Self::Network => write!(f, "Network"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    OrphanedRequirement,
    OrphanedDesign,
    OrphanedTask,
    MissingLink,
    CircularDependency,
    InvalidEarsFormat,
    InconsistentStatus,
}

impl std::fmt::Display for ValidationErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OrphanedRequirement => write!(f, "orphaned-requirement"),
            Self::OrphanedDesign => write!(f, "orphaned-design"),
            Self::OrphanedTask => write!(f, "orphaned-task"),
            Self::MissingLink => write!(f, "missing-link"),
            Self::CircularDependency => write!(f, "circular-dependency"),
            Self::InvalidEarsFormat => write!(f, "invalid-ears-format"),
            Self::InconsistentStatus => write!(f, "inconsistent-status"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelinePhase {
    Requirements,
    Design,
    Tasks,
    Implementation,
    Verification,
}

impl std::fmt::Display for PipelinePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Requirements => write!(f, "requirements"),
            Self::Design => write!(f, "design"),
            Self::Tasks => write!(f, "tasks"),
            Self::Implementation => write!(f, "implementation"),
            Self::Verification => write!(f, "verification"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpecError {
    InvalidEarsFormat(String),
    RequirementNotFound(String),
    DesignNotFound(String),
    TaskNotFound(String),
    DuplicateId(String),
    CircularDependency(String),
    LinkError(String),
    InitError(String),
    ValidationFailed(String),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEarsFormat(msg) => write!(f, "invalid EARS format: {msg}"),
            Self::RequirementNotFound(id) => write!(f, "requirement not found: {id}"),
            Self::DesignNotFound(id) => write!(f, "design not found: {id}"),
            Self::TaskNotFound(id) => write!(f, "task not found: {id}"),
            Self::DuplicateId(id) => write!(f, "duplicate ID: {id}"),
            Self::CircularDependency(msg) => write!(f, "circular dependency: {msg}"),
            Self::LinkError(msg) => write!(f, "link error: {msg}"),
            Self::InitError(msg) => write!(f, "init error: {msg}"),
            Self::ValidationFailed(msg) => write!(f, "validation failed: {msg}"),
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub spec_dir: String,
    pub requirements_file: String,
    pub design_file: String,
    pub tasks_file: String,
    pub auto_validate: bool,
    pub track_progress: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            spec_dir: ".spec".to_string(),
            requirements_file: "requirements.md".to_string(),
            design_file: "design.md".to_string(),
            tasks_file: "tasks.md".to_string(),
            auto_validate: true,
            track_progress: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EarsRequirement {
    pub id: String,
    pub pattern: EarsPattern,
    pub text: String,
    pub system: String,
    pub action: String,
    pub trigger: Option<String>,
    pub condition: Option<String>,
    pub feature: Option<String>,
    pub priority: Priority,
    pub status: RequirementStatus,
    pub linked_design_ids: Vec<String>,
    pub linked_task_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InterfaceSpec {
    pub name: String,
    pub interface_type: InterfaceType,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DataFlowStep {
    pub from: String,
    pub to: String,
    pub data_type: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DesignDecision {
    pub id: String,
    pub title: String,
    pub description: String,
    pub component: String,
    pub interfaces: Vec<InterfaceSpec>,
    pub data_flow: Vec<DataFlowStep>,
    pub rationale: String,
    pub alternatives: Vec<String>,
    pub linked_requirement_ids: Vec<String>,
    pub linked_task_ids: Vec<String>,
    pub status: DesignStatus,
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub effort: Effort,
    pub dependencies: Vec<String>,
    pub linked_requirement_ids: Vec<String>,
    pub linked_design_ids: Vec<String>,
    pub status: TaskStatus,
    pub order: u32,
    pub assignee: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub source_id: String,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub message: String,
    pub source_id: String,
}

#[derive(Debug, Clone)]
pub struct SpecCoverage {
    pub requirements_with_design: usize,
    pub requirements_without_design: usize,
    pub designs_with_tasks: usize,
    pub designs_without_tasks: usize,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub coverage_percent: f64,
}

#[derive(Debug, Clone)]
pub struct SpecValidation {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub coverage: SpecCoverage,
}

#[derive(Debug, Clone)]
pub struct SpecSummary {
    pub total_requirements: usize,
    pub total_designs: usize,
    pub total_tasks: usize,
    pub phase: PipelinePhase,
    pub coverage: SpecCoverage,
    pub validation: SpecValidation,
}

// === Pipeline ===

#[derive(Debug, Clone)]
pub struct SpecPipeline {
    pub config: PipelineConfig,
    pub requirements: HashMap<String, EarsRequirement>,
    pub designs: HashMap<String, DesignDecision>,
    pub tasks: HashMap<String, TaskItem>,
    next_req_id: u32,
    next_design_id: u32,
    next_task_id: u32,
    initialized: bool,
}

impl SpecPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            requirements: HashMap::new(),
            designs: HashMap::new(),
            tasks: HashMap::new(),
            next_req_id: 1,
            next_design_id: 1,
            next_task_id: 1,
            initialized: false,
        }
    }

    pub fn init_spec(&mut self) -> Result<(), SpecError> {
        if self.initialized {
            return Err(SpecError::InitError("pipeline already initialized".to_string()));
        }
        self.initialized = true;
        Ok(())
    }

    // --- Requirement operations ---

    pub fn next_requirement_id(&self) -> String {
        format!("REQ-{:03}", self.next_req_id)
    }

    pub fn add_requirement(&mut self, mut req: EarsRequirement) -> Result<String, SpecError> {
        if req.id.is_empty() {
            req.id = self.next_requirement_id();
        }
        if self.requirements.contains_key(&req.id) {
            return Err(SpecError::DuplicateId(req.id));
        }
        let id = req.id.clone();
        self.requirements.insert(id.clone(), req);
        self.next_req_id += 1;
        Ok(id)
    }

    pub fn get_requirement(&self, id: &str) -> Result<&EarsRequirement, SpecError> {
        self.requirements
            .get(id)
            .ok_or_else(|| SpecError::RequirementNotFound(id.to_string()))
    }

    // --- EARS parsing ---

    pub fn detect_ears_pattern(text: &str) -> Option<EarsPattern> {
        let trimmed = text.trim();
        if trimmed.starts_with("When ") {
            Some(EarsPattern::EventDriven)
        } else if trimmed.starts_with("If ") {
            Some(EarsPattern::UnwantedBehavior)
        } else if trimmed.starts_with("While ") {
            Some(EarsPattern::StateDriven)
        } else if trimmed.starts_with("Where ") {
            Some(EarsPattern::Optional)
        } else if trimmed.contains(" shall ") {
            Some(EarsPattern::Ubiquitous)
        } else {
            None
        }
    }

    pub fn parse_ears(&self, text: &str) -> Result<EarsRequirement, SpecError> {
        let trimmed = text.trim();
        let pattern = Self::detect_ears_pattern(trimmed)
            .ok_or_else(|| SpecError::InvalidEarsFormat(
                "text does not match any EARS pattern (must contain ' shall ')".to_string(),
            ))?;

        let (system, action, trigger, condition, feature) = match &pattern {
            EarsPattern::Ubiquitous => {
                let (sys, act) = Self::extract_system_action(trimmed)?;
                (sys, act, None, None, None)
            }
            EarsPattern::EventDriven => {
                let clause = Self::extract_prefix_clause(trimmed, "When ", ", the ")?;
                let remainder = Self::after_prefix_clause(trimmed, ", the ")?;
                let full = format!("the {remainder}");
                let (sys, act) = Self::extract_system_action(&full)?;
                (sys, act, Some(clause), None, None)
            }
            EarsPattern::UnwantedBehavior => {
                let clause = Self::extract_prefix_clause(trimmed, "If ", ", then the ")?;
                let remainder = Self::after_prefix_clause(trimmed, ", then the ")?;
                let full = format!("the {remainder}");
                let (sys, act) = Self::extract_system_action(&full)?;
                (sys, act, None, Some(clause), None)
            }
            EarsPattern::StateDriven => {
                let clause = Self::extract_prefix_clause(trimmed, "While ", ", the ")?;
                let remainder = Self::after_prefix_clause(trimmed, ", the ")?;
                let full = format!("the {remainder}");
                let (sys, act) = Self::extract_system_action(&full)?;
                (sys, act, None, None, Some(clause))
            }
            EarsPattern::Optional => {
                let clause = Self::extract_prefix_clause(trimmed, "Where ", ", the ")?;
                let remainder = Self::after_prefix_clause(trimmed, ", the ")?;
                let full = format!("the {remainder}");
                let (sys, act) = Self::extract_system_action(&full)?;
                (sys, act, None, None, Some(clause))
            }
        };

        Ok(EarsRequirement {
            id: String::new(),
            pattern,
            text: trimmed.to_string(),
            system,
            action,
            trigger,
            condition,
            feature,
            priority: Priority::Must,
            status: RequirementStatus::Draft,
            linked_design_ids: Vec::new(),
            linked_task_ids: Vec::new(),
        })
    }

    fn extract_system_action(text: &str) -> Result<(String, String), SpecError> {
        // Expects "... the [system] shall [action]" (case-insensitive "the")
        let lower = text.to_lowercase();
        let the_idx = lower.find("the ").ok_or_else(|| {
            SpecError::InvalidEarsFormat("missing 'the [system]' clause".to_string())
        })?;
        let after_the = &text[the_idx + 4..];
        let shall_idx = after_the.find(" shall ").ok_or_else(|| {
            SpecError::InvalidEarsFormat("missing ' shall ' keyword".to_string())
        })?;
        let system = after_the[..shall_idx].trim().to_string();
        let action = after_the[shall_idx + 7..].trim().to_string();
        if system.is_empty() {
            return Err(SpecError::InvalidEarsFormat("empty system name".to_string()));
        }
        if action.is_empty() {
            return Err(SpecError::InvalidEarsFormat("empty action".to_string()));
        }
        Ok((system, action))
    }

    fn extract_prefix_clause(text: &str, prefix: &str, separator: &str) -> Result<String, SpecError> {
        let start = prefix.len();
        let sep_idx = text.find(separator).ok_or_else(|| {
            SpecError::InvalidEarsFormat(format!("missing separator '{separator}' after prefix '{prefix}'"))
        })?;
        let clause = text[start..sep_idx].trim().to_string();
        if clause.is_empty() {
            return Err(SpecError::InvalidEarsFormat("empty prefix clause".to_string()));
        }
        Ok(clause)
    }

    fn after_prefix_clause<'a>(text: &'a str, separator: &str) -> Result<&'a str, SpecError> {
        let sep_idx = text.find(separator).ok_or_else(|| {
            SpecError::InvalidEarsFormat(format!("missing separator '{separator}'"))
        })?;
        Ok(&text[sep_idx + separator.len()..])
    }

    // --- Design operations ---

    pub fn next_design_id(&self) -> String {
        format!("DES-{:03}", self.next_design_id)
    }

    pub fn add_design(&mut self, mut design: DesignDecision) -> Result<String, SpecError> {
        if design.id.is_empty() {
            design.id = self.next_design_id();
        }
        if self.designs.contains_key(&design.id) {
            return Err(SpecError::DuplicateId(design.id));
        }
        let id = design.id.clone();
        self.designs.insert(id.clone(), design);
        self.next_design_id += 1;
        Ok(id)
    }

    pub fn get_design(&self, id: &str) -> Result<&DesignDecision, SpecError> {
        self.designs
            .get(id)
            .ok_or_else(|| SpecError::DesignNotFound(id.to_string()))
    }

    // --- Task operations ---

    pub fn next_task_id(&self) -> String {
        format!("TASK-{:03}", self.next_task_id)
    }

    pub fn add_task(&mut self, mut task: TaskItem) -> Result<String, SpecError> {
        if task.id.is_empty() {
            task.id = self.next_task_id();
        }
        if self.tasks.contains_key(&task.id) {
            return Err(SpecError::DuplicateId(task.id));
        }
        let id = task.id.clone();
        self.tasks.insert(id.clone(), task);
        self.next_task_id += 1;
        Ok(id)
    }

    pub fn get_task(&self, id: &str) -> Result<&TaskItem, SpecError> {
        self.tasks
            .get(id)
            .ok_or_else(|| SpecError::TaskNotFound(id.to_string()))
    }

    pub fn update_task_status(&mut self, id: &str, status: TaskStatus) -> Result<(), SpecError> {
        let task = self.tasks.get_mut(id)
            .ok_or_else(|| SpecError::TaskNotFound(id.to_string()))?;
        task.status = status;
        Ok(())
    }

    // --- Linking ---

    pub fn link_requirement_to_design(
        &mut self,
        req_id: &str,
        design_id: &str,
    ) -> Result<(), SpecError> {
        if !self.requirements.contains_key(req_id) {
            return Err(SpecError::RequirementNotFound(req_id.to_string()));
        }
        if !self.designs.contains_key(design_id) {
            return Err(SpecError::DesignNotFound(design_id.to_string()));
        }
        let req = self.requirements.get_mut(req_id).expect("checked above");
        if !req.linked_design_ids.contains(&design_id.to_string()) {
            req.linked_design_ids.push(design_id.to_string());
        }
        let design = self.designs.get_mut(design_id).expect("checked above");
        if !design.linked_requirement_ids.contains(&req_id.to_string()) {
            design.linked_requirement_ids.push(req_id.to_string());
        }
        Ok(())
    }

    pub fn link_design_to_task(
        &mut self,
        design_id: &str,
        task_id: &str,
    ) -> Result<(), SpecError> {
        if !self.designs.contains_key(design_id) {
            return Err(SpecError::DesignNotFound(design_id.to_string()));
        }
        if !self.tasks.contains_key(task_id) {
            return Err(SpecError::TaskNotFound(task_id.to_string()));
        }
        let design = self.designs.get_mut(design_id).expect("checked above");
        if !design.linked_task_ids.contains(&task_id.to_string()) {
            design.linked_task_ids.push(task_id.to_string());
        }
        let task = self.tasks.get_mut(task_id).expect("checked above");
        if !task.linked_design_ids.contains(&design_id.to_string()) {
            task.linked_design_ids.push(design_id.to_string());
        }
        Ok(())
    }

    // --- Validation ---

    pub fn validate(&self) -> SpecValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check orphaned requirements (no linked design)
        for (id, req) in &self.requirements {
            if req.linked_design_ids.is_empty() {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::OrphanedRequirement,
                    message: format!("requirement {id} has no linked design"),
                    source_id: id.clone(),
                });
            }
        }

        // Check orphaned designs (no linked requirement)
        for (id, design) in &self.designs {
            if design.linked_requirement_ids.is_empty() {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::OrphanedDesign,
                    message: format!("design {id} has no linked requirement"),
                    source_id: id.clone(),
                });
            }
            if design.linked_task_ids.is_empty() {
                warnings.push(ValidationWarning {
                    message: format!("design {id} has no linked tasks"),
                    source_id: id.clone(),
                });
            }
        }

        // Check orphaned tasks (no linked design)
        for (id, task) in &self.tasks {
            if task.linked_design_ids.is_empty() {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::OrphanedTask,
                    message: format!("task {id} has no linked design"),
                    source_id: id.clone(),
                });
            }
        }

        // Check circular task dependencies
        for (id, task) in &self.tasks {
            for dep_id in &task.dependencies {
                if let Some(dep_task) = self.tasks.get(dep_id) {
                    if dep_task.dependencies.contains(id) {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::CircularDependency,
                            message: format!("circular dependency between {id} and {dep_id}"),
                            source_id: id.clone(),
                        });
                    }
                } else {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::MissingLink,
                        message: format!("task {id} depends on non-existent task {dep_id}"),
                        source_id: id.clone(),
                    });
                }
            }
        }

        let coverage = self.calculate_coverage();
        let is_valid = errors.is_empty();

        SpecValidation {
            is_valid,
            errors,
            warnings,
            coverage,
        }
    }

    fn calculate_coverage(&self) -> SpecCoverage {
        let requirements_with_design = self
            .requirements
            .values()
            .filter(|r| !r.linked_design_ids.is_empty())
            .count();
        let requirements_without_design = self.requirements.len() - requirements_with_design;

        let designs_with_tasks = self
            .designs
            .values()
            .filter(|d| !d.linked_task_ids.is_empty())
            .count();
        let designs_without_tasks = self.designs.len() - designs_with_tasks;

        let tasks_completed = self
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        let tasks_total = self.tasks.len();

        let total_items = self.requirements.len() + self.designs.len() + tasks_total;
        let linked_items = requirements_with_design + designs_with_tasks + tasks_completed;
        let coverage_percent = if total_items == 0 {
            0.0
        } else {
            (linked_items as f64 / total_items as f64) * 100.0
        };

        SpecCoverage {
            requirements_with_design,
            requirements_without_design,
            designs_with_tasks,
            designs_without_tasks,
            tasks_completed,
            tasks_total,
            coverage_percent,
        }
    }

    // --- Summary ---

    pub fn get_summary(&self) -> SpecSummary {
        let validation = self.validate();
        let coverage = self.calculate_coverage();

        let phase = if self.requirements.is_empty() {
            PipelinePhase::Requirements
        } else if self.designs.is_empty() {
            PipelinePhase::Design
        } else if self.tasks.is_empty() {
            PipelinePhase::Tasks
        } else if self.tasks.values().all(|t| t.status == TaskStatus::Done) {
            PipelinePhase::Verification
        } else {
            PipelinePhase::Implementation
        };

        SpecSummary {
            total_requirements: self.requirements.len(),
            total_designs: self.designs.len(),
            total_tasks: self.tasks.len(),
            phase,
            coverage,
            validation,
        }
    }

    // --- Next tasks ---

    pub fn get_next_tasks(&self) -> Vec<&TaskItem> {
        self.tasks
            .values()
            .filter(|task| {
                task.status == TaskStatus::Todo
                    && task.dependencies.iter().all(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .map(|d| d.status == TaskStatus::Done)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    // --- Markdown generation ---

    pub fn generate_requirements_md(&self) -> String {
        let mut md = String::from("# Requirements\n\n");
        let mut reqs: Vec<&EarsRequirement> = self.requirements.values().collect();
        reqs.sort_by(|a, b| a.id.cmp(&b.id));

        for req in reqs {
            md.push_str(&format!("## {} [{}] [{}]\n\n", req.id, req.priority, req.status));
            md.push_str(&format!("**Pattern:** {}\n\n", req.pattern));
            md.push_str(&format!("> {}\n\n", req.text));
            md.push_str(&format!("- **System:** {}\n", req.system));
            md.push_str(&format!("- **Action:** {}\n", req.action));
            if let Some(trigger) = &req.trigger {
                md.push_str(&format!("- **Trigger:** {trigger}\n"));
            }
            if let Some(condition) = &req.condition {
                md.push_str(&format!("- **Condition:** {condition}\n"));
            }
            if let Some(feature) = &req.feature {
                md.push_str(&format!("- **Feature:** {feature}\n"));
            }
            if !req.linked_design_ids.is_empty() {
                md.push_str(&format!("- **Designs:** {}\n", req.linked_design_ids.join(", ")));
            }
            md.push('\n');
        }
        md
    }

    pub fn generate_design_md(&self) -> String {
        let mut md = String::from("# Design Decisions\n\n");
        let mut designs: Vec<&DesignDecision> = self.designs.values().collect();
        designs.sort_by(|a, b| a.id.cmp(&b.id));

        for design in designs {
            md.push_str(&format!("## {} — {} [{}]\n\n", design.id, design.title, design.status));
            md.push_str(&format!("{}\n\n", design.description));
            md.push_str(&format!("**Component:** {}\n\n", design.component));
            md.push_str(&format!("**Rationale:** {}\n\n", design.rationale));
            if !design.alternatives.is_empty() {
                md.push_str("**Alternatives considered:**\n");
                for alt in &design.alternatives {
                    md.push_str(&format!("- {alt}\n"));
                }
                md.push('\n');
            }
            if !design.interfaces.is_empty() {
                md.push_str("**Interfaces:**\n");
                for iface in &design.interfaces {
                    md.push_str(&format!("- {} ({}) — {}\n", iface.name, iface.interface_type, iface.description));
                }
                md.push('\n');
            }
            if !design.linked_requirement_ids.is_empty() {
                md.push_str(&format!(
                    "**Requirements:** {}\n\n",
                    design.linked_requirement_ids.join(", ")
                ));
            }
        }
        md
    }

    pub fn generate_tasks_md(&self) -> String {
        let mut md = String::from("# Tasks\n\n");
        let mut tasks: Vec<&TaskItem> = self.tasks.values().collect();
        tasks.sort_by_key(|t| t.order);

        for task in tasks {
            let check = if task.status == TaskStatus::Done { "x" } else { " " };
            md.push_str(&format!(
                "- [{}] **{}** — {} [{}] [{}]\n",
                check, task.id, task.title, task.effort, task.status
            ));
            if !task.description.is_empty() {
                md.push_str(&format!("  {}\n", task.description));
            }
            if !task.dependencies.is_empty() {
                md.push_str(&format!("  Depends on: {}\n", task.dependencies.join(", ")));
            }
            if let Some(assignee) = &task.assignee {
                md.push_str(&format!("  Assignee: {assignee}\n"));
            }
        }
        md
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pipeline() -> SpecPipeline {
        SpecPipeline::new(PipelineConfig::default())
    }

    fn make_requirement(text: &str) -> EarsRequirement {
        EarsRequirement {
            id: String::new(),
            pattern: EarsPattern::Ubiquitous,
            text: text.to_string(),
            system: "system".to_string(),
            action: "do something".to_string(),
            trigger: None,
            condition: None,
            feature: None,
            priority: Priority::Must,
            status: RequirementStatus::Draft,
            linked_design_ids: Vec::new(),
            linked_task_ids: Vec::new(),
        }
    }

    fn make_design(title: &str) -> DesignDecision {
        DesignDecision {
            id: String::new(),
            title: title.to_string(),
            description: "A design".to_string(),
            component: "core".to_string(),
            interfaces: Vec::new(),
            data_flow: Vec::new(),
            rationale: "Best approach".to_string(),
            alternatives: Vec::new(),
            linked_requirement_ids: Vec::new(),
            linked_task_ids: Vec::new(),
            status: DesignStatus::Proposed,
        }
    }

    fn make_task(title: &str, order: u32) -> TaskItem {
        TaskItem {
            id: String::new(),
            title: title.to_string(),
            description: String::new(),
            effort: Effort::Medium,
            dependencies: Vec::new(),
            linked_requirement_ids: Vec::new(),
            linked_design_ids: Vec::new(),
            status: TaskStatus::Todo,
            order,
            assignee: None,
        }
    }

    // --- EARS pattern detection ---

    #[test]
    fn test_detect_ubiquitous_pattern() {
        let result = SpecPipeline::detect_ears_pattern("The system shall do X");
        assert_eq!(result, Some(EarsPattern::Ubiquitous));
    }

    #[test]
    fn test_detect_event_driven_pattern() {
        let result = SpecPipeline::detect_ears_pattern("When a user logs in, the system shall send a welcome email");
        assert_eq!(result, Some(EarsPattern::EventDriven));
    }

    #[test]
    fn test_detect_unwanted_behavior_pattern() {
        let result = SpecPipeline::detect_ears_pattern("If the connection is lost, then the system shall retry");
        assert_eq!(result, Some(EarsPattern::UnwantedBehavior));
    }

    #[test]
    fn test_detect_state_driven_pattern() {
        let result = SpecPipeline::detect_ears_pattern("While the system is in maintenance mode, the system shall reject requests");
        assert_eq!(result, Some(EarsPattern::StateDriven));
    }

    #[test]
    fn test_detect_optional_pattern() {
        let result = SpecPipeline::detect_ears_pattern("Where premium features are enabled, the system shall show analytics");
        assert_eq!(result, Some(EarsPattern::Optional));
    }

    #[test]
    fn test_detect_no_pattern() {
        let result = SpecPipeline::detect_ears_pattern("This is not a requirement");
        assert_eq!(result, None);
    }

    // --- EARS parsing ---

    #[test]
    fn test_parse_ubiquitous() {
        let pipeline = make_pipeline();
        let req = pipeline.parse_ears("The editor shall highlight syntax errors").unwrap();
        assert_eq!(req.pattern, EarsPattern::Ubiquitous);
        assert_eq!(req.system, "editor");
        assert_eq!(req.action, "highlight syntax errors");
        assert!(req.trigger.is_none());
        assert!(req.condition.is_none());
        assert!(req.feature.is_none());
    }

    #[test]
    fn test_parse_event_driven() {
        let pipeline = make_pipeline();
        let req = pipeline
            .parse_ears("When a file is saved, the editor shall run linting")
            .unwrap();
        assert_eq!(req.pattern, EarsPattern::EventDriven);
        assert_eq!(req.trigger, Some("a file is saved".to_string()));
        assert_eq!(req.system, "editor");
        assert_eq!(req.action, "run linting");
    }

    #[test]
    fn test_parse_unwanted_behavior() {
        let pipeline = make_pipeline();
        let req = pipeline
            .parse_ears("If the API returns 500, then the system shall retry the request")
            .unwrap();
        assert_eq!(req.pattern, EarsPattern::UnwantedBehavior);
        assert_eq!(req.condition, Some("the API returns 500".to_string()));
        assert_eq!(req.system, "system");
        assert_eq!(req.action, "retry the request");
    }

    #[test]
    fn test_parse_state_driven() {
        let pipeline = make_pipeline();
        let req = pipeline
            .parse_ears("While the system is offline, the editor shall cache changes locally")
            .unwrap();
        assert_eq!(req.pattern, EarsPattern::StateDriven);
        assert_eq!(req.feature, Some("the system is offline".to_string()));
        assert_eq!(req.system, "editor");
        assert_eq!(req.action, "cache changes locally");
    }

    #[test]
    fn test_parse_optional() {
        let pipeline = make_pipeline();
        let req = pipeline
            .parse_ears("Where dark mode is enabled, the editor shall use dark theme colors")
            .unwrap();
        assert_eq!(req.pattern, EarsPattern::Optional);
        assert_eq!(req.feature, Some("dark mode is enabled".to_string()));
        assert_eq!(req.system, "editor");
        assert_eq!(req.action, "use dark theme colors");
    }

    #[test]
    fn test_parse_invalid_no_shall() {
        let pipeline = make_pipeline();
        let result = pipeline.parse_ears("The editor does something");
        assert!(result.is_err());
        assert!(matches!(result, Err(SpecError::InvalidEarsFormat(_))));
    }

    #[test]
    fn test_parse_invalid_empty_system() {
        let pipeline = make_pipeline();
        let result = pipeline.parse_ears("The  shall do something");
        assert!(result.is_err());
    }

    // --- Requirement CRUD ---

    #[test]
    fn test_add_requirement() {
        let mut pipeline = make_pipeline();
        let id = pipeline.add_requirement(make_requirement("test req")).unwrap();
        assert_eq!(id, "REQ-001");
        assert!(pipeline.get_requirement("REQ-001").is_ok());
    }

    #[test]
    fn test_add_multiple_requirements_auto_increment() {
        let mut pipeline = make_pipeline();
        let id1 = pipeline.add_requirement(make_requirement("first")).unwrap();
        let id2 = pipeline.add_requirement(make_requirement("second")).unwrap();
        let id3 = pipeline.add_requirement(make_requirement("third")).unwrap();
        assert_eq!(id1, "REQ-001");
        assert_eq!(id2, "REQ-002");
        assert_eq!(id3, "REQ-003");
    }

    #[test]
    fn test_duplicate_requirement_id() {
        let mut pipeline = make_pipeline();
        let mut req = make_requirement("test");
        req.id = "REQ-001".to_string();
        pipeline.add_requirement(req.clone()).unwrap();
        let result = pipeline.add_requirement(req);
        assert!(matches!(result, Err(SpecError::DuplicateId(_))));
    }

    #[test]
    fn test_get_nonexistent_requirement() {
        let pipeline = make_pipeline();
        let result = pipeline.get_requirement("REQ-999");
        assert!(matches!(result, Err(SpecError::RequirementNotFound(_))));
    }

    #[test]
    fn test_next_requirement_id() {
        let pipeline = make_pipeline();
        assert_eq!(pipeline.next_requirement_id(), "REQ-001");
    }

    // --- Design CRUD ---

    #[test]
    fn test_add_design() {
        let mut pipeline = make_pipeline();
        let id = pipeline.add_design(make_design("Auth module")).unwrap();
        assert_eq!(id, "DES-001");
        assert!(pipeline.get_design("DES-001").is_ok());
    }

    #[test]
    fn test_add_multiple_designs_auto_increment() {
        let mut pipeline = make_pipeline();
        let id1 = pipeline.add_design(make_design("first")).unwrap();
        let id2 = pipeline.add_design(make_design("second")).unwrap();
        assert_eq!(id1, "DES-001");
        assert_eq!(id2, "DES-002");
    }

    #[test]
    fn test_duplicate_design_id() {
        let mut pipeline = make_pipeline();
        let mut design = make_design("test");
        design.id = "DES-001".to_string();
        pipeline.add_design(design.clone()).unwrap();
        let result = pipeline.add_design(design);
        assert!(matches!(result, Err(SpecError::DuplicateId(_))));
    }

    #[test]
    fn test_get_nonexistent_design() {
        let pipeline = make_pipeline();
        assert!(matches!(pipeline.get_design("DES-999"), Err(SpecError::DesignNotFound(_))));
    }

    // --- Task CRUD ---

    #[test]
    fn test_add_task() {
        let mut pipeline = make_pipeline();
        let id = pipeline.add_task(make_task("Implement auth", 1)).unwrap();
        assert_eq!(id, "TASK-001");
        assert!(pipeline.get_task("TASK-001").is_ok());
    }

    #[test]
    fn test_add_multiple_tasks_auto_increment() {
        let mut pipeline = make_pipeline();
        let id1 = pipeline.add_task(make_task("first", 1)).unwrap();
        let id2 = pipeline.add_task(make_task("second", 2)).unwrap();
        let id3 = pipeline.add_task(make_task("third", 3)).unwrap();
        assert_eq!(id1, "TASK-001");
        assert_eq!(id2, "TASK-002");
        assert_eq!(id3, "TASK-003");
    }

    #[test]
    fn test_duplicate_task_id() {
        let mut pipeline = make_pipeline();
        let mut task = make_task("test", 1);
        task.id = "TASK-001".to_string();
        pipeline.add_task(task.clone()).unwrap();
        let result = pipeline.add_task(task);
        assert!(matches!(result, Err(SpecError::DuplicateId(_))));
    }

    #[test]
    fn test_get_nonexistent_task() {
        let pipeline = make_pipeline();
        assert!(matches!(pipeline.get_task("TASK-999"), Err(SpecError::TaskNotFound(_))));
    }

    #[test]
    fn test_update_task_status() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("work", 1)).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::InProgress).unwrap();
        assert_eq!(pipeline.get_task("TASK-001").unwrap().status, TaskStatus::InProgress);
    }

    #[test]
    fn test_update_task_status_not_found() {
        let mut pipeline = make_pipeline();
        let result = pipeline.update_task_status("TASK-999", TaskStatus::Done);
        assert!(matches!(result, Err(SpecError::TaskNotFound(_))));
    }

    // --- Linking ---

    #[test]
    fn test_link_requirement_to_design() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.link_requirement_to_design("REQ-001", "DES-001").unwrap();

        let req = pipeline.get_requirement("REQ-001").unwrap();
        assert!(req.linked_design_ids.contains(&"DES-001".to_string()));

        let design = pipeline.get_design("DES-001").unwrap();
        assert!(design.linked_requirement_ids.contains(&"REQ-001".to_string()));
    }

    #[test]
    fn test_link_design_to_task() {
        let mut pipeline = make_pipeline();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.add_task(make_task("task", 1)).unwrap();
        pipeline.link_design_to_task("DES-001", "TASK-001").unwrap();

        let design = pipeline.get_design("DES-001").unwrap();
        assert!(design.linked_task_ids.contains(&"TASK-001".to_string()));

        let task = pipeline.get_task("TASK-001").unwrap();
        assert!(task.linked_design_ids.contains(&"DES-001".to_string()));
    }

    #[test]
    fn test_link_idempotent() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.link_requirement_to_design("REQ-001", "DES-001").unwrap();
        pipeline.link_requirement_to_design("REQ-001", "DES-001").unwrap();

        let req = pipeline.get_requirement("REQ-001").unwrap();
        assert_eq!(req.linked_design_ids.len(), 1);
    }

    #[test]
    fn test_link_requirement_not_found() {
        let mut pipeline = make_pipeline();
        pipeline.add_design(make_design("design")).unwrap();
        let result = pipeline.link_requirement_to_design("REQ-999", "DES-001");
        assert!(matches!(result, Err(SpecError::RequirementNotFound(_))));
    }

    #[test]
    fn test_link_design_not_found() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        let result = pipeline.link_requirement_to_design("REQ-001", "DES-999");
        assert!(matches!(result, Err(SpecError::DesignNotFound(_))));
    }

    #[test]
    fn test_link_design_to_task_not_found() {
        let mut pipeline = make_pipeline();
        pipeline.add_design(make_design("design")).unwrap();
        let result = pipeline.link_design_to_task("DES-001", "TASK-999");
        assert!(matches!(result, Err(SpecError::TaskNotFound(_))));
    }

    // --- Validation ---

    #[test]
    fn test_validate_empty_pipeline() {
        let pipeline = make_pipeline();
        let validation = pipeline.validate();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_validate_orphaned_requirement() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("orphan")).unwrap();
        let validation = pipeline.validate();
        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.error_type == ValidationErrorType::OrphanedRequirement));
    }

    #[test]
    fn test_validate_orphaned_design() {
        let mut pipeline = make_pipeline();
        pipeline.add_design(make_design("orphan design")).unwrap();
        let validation = pipeline.validate();
        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.error_type == ValidationErrorType::OrphanedDesign));
    }

    #[test]
    fn test_validate_orphaned_task() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("orphan task", 1)).unwrap();
        let validation = pipeline.validate();
        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.error_type == ValidationErrorType::OrphanedTask));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let mut pipeline = make_pipeline();
        let mut t1 = make_task("task A", 1);
        t1.id = "TASK-001".to_string();
        t1.dependencies = vec!["TASK-002".to_string()];
        let mut t2 = make_task("task B", 2);
        t2.id = "TASK-002".to_string();
        t2.dependencies = vec!["TASK-001".to_string()];
        pipeline.add_task(t1).unwrap();
        pipeline.add_task(t2).unwrap();

        let validation = pipeline.validate();
        assert!(validation.errors.iter().any(|e| e.error_type == ValidationErrorType::CircularDependency));
    }

    #[test]
    fn test_validate_missing_dependency_link() {
        let mut pipeline = make_pipeline();
        let mut task = make_task("depends on ghost", 1);
        task.dependencies = vec!["TASK-999".to_string()];
        pipeline.add_task(task).unwrap();

        let validation = pipeline.validate();
        assert!(validation.errors.iter().any(|e| e.error_type == ValidationErrorType::MissingLink));
    }

    #[test]
    fn test_validate_fully_linked_pipeline() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.add_task(make_task("task", 1)).unwrap();
        pipeline.link_requirement_to_design("REQ-001", "DES-001").unwrap();
        pipeline.link_design_to_task("DES-001", "TASK-001").unwrap();

        let validation = pipeline.validate();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }

    // --- Coverage ---

    #[test]
    fn test_coverage_empty() {
        let pipeline = make_pipeline();
        let coverage = pipeline.calculate_coverage();
        assert_eq!(coverage.coverage_percent, 0.0);
        assert_eq!(coverage.tasks_total, 0);
    }

    #[test]
    fn test_coverage_partial() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("linked")).unwrap();
        pipeline.add_requirement(make_requirement("unlinked")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.link_requirement_to_design("REQ-001", "DES-001").unwrap();

        let coverage = pipeline.calculate_coverage();
        assert_eq!(coverage.requirements_with_design, 1);
        assert_eq!(coverage.requirements_without_design, 1);
    }

    #[test]
    fn test_coverage_with_completed_tasks() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("done", 1)).unwrap();
        pipeline.add_task(make_task("todo", 2)).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::Done).unwrap();

        let coverage = pipeline.calculate_coverage();
        assert_eq!(coverage.tasks_completed, 1);
        assert_eq!(coverage.tasks_total, 2);
    }

    // --- Next tasks ---

    #[test]
    fn test_get_next_tasks_no_deps() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("ready", 1)).unwrap();
        pipeline.add_task(make_task("also ready", 2)).unwrap();

        let next = pipeline.get_next_tasks();
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_get_next_tasks_with_deps() {
        let mut pipeline = make_pipeline();
        let mut t1 = make_task("first", 1);
        t1.id = "TASK-001".to_string();
        let mut t2 = make_task("second", 2);
        t2.id = "TASK-002".to_string();
        t2.dependencies = vec!["TASK-001".to_string()];
        pipeline.add_task(t1).unwrap();
        pipeline.add_task(t2).unwrap();

        let next = pipeline.get_next_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "TASK-001");
    }

    #[test]
    fn test_get_next_tasks_deps_satisfied() {
        let mut pipeline = make_pipeline();
        let mut t1 = make_task("first", 1);
        t1.id = "TASK-001".to_string();
        let mut t2 = make_task("second", 2);
        t2.id = "TASK-002".to_string();
        t2.dependencies = vec!["TASK-001".to_string()];
        pipeline.add_task(t1).unwrap();
        pipeline.add_task(t2).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::Done).unwrap();

        let next = pipeline.get_next_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "TASK-002");
    }

    #[test]
    fn test_get_next_tasks_excludes_non_todo() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("in progress", 1)).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::InProgress).unwrap();

        let next = pipeline.get_next_tasks();
        assert!(next.is_empty());
    }

    // --- Markdown generation ---

    #[test]
    fn test_generate_requirements_md() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("The system shall do X")).unwrap();
        let md = pipeline.generate_requirements_md();
        assert!(md.contains("# Requirements"));
        assert!(md.contains("REQ-001"));
        assert!(md.contains("The system shall do X"));
    }

    #[test]
    fn test_generate_design_md() {
        let mut pipeline = make_pipeline();
        pipeline.add_design(make_design("Auth module")).unwrap();
        let md = pipeline.generate_design_md();
        assert!(md.contains("# Design Decisions"));
        assert!(md.contains("DES-001"));
        assert!(md.contains("Auth module"));
    }

    #[test]
    fn test_generate_tasks_md() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("Write tests", 1)).unwrap();
        let md = pipeline.generate_tasks_md();
        assert!(md.contains("# Tasks"));
        assert!(md.contains("TASK-001"));
        assert!(md.contains("Write tests"));
    }

    #[test]
    fn test_generate_tasks_md_done_checkbox() {
        let mut pipeline = make_pipeline();
        pipeline.add_task(make_task("Done task", 1)).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::Done).unwrap();
        let md = pipeline.generate_tasks_md();
        assert!(md.contains("[x]"));
    }

    // --- Summary ---

    #[test]
    fn test_summary_requirements_phase() {
        let pipeline = make_pipeline();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Requirements);
    }

    #[test]
    fn test_summary_design_phase() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Design);
    }

    #[test]
    fn test_summary_tasks_phase() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Tasks);
    }

    #[test]
    fn test_summary_implementation_phase() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.add_task(make_task("task", 1)).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Implementation);
    }

    #[test]
    fn test_summary_verification_phase() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("req")).unwrap();
        pipeline.add_design(make_design("design")).unwrap();
        pipeline.add_task(make_task("task", 1)).unwrap();
        pipeline.update_task_status("TASK-001", TaskStatus::Done).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Verification);
    }

    #[test]
    fn test_summary_totals() {
        let mut pipeline = make_pipeline();
        pipeline.add_requirement(make_requirement("r1")).unwrap();
        pipeline.add_requirement(make_requirement("r2")).unwrap();
        pipeline.add_design(make_design("d1")).unwrap();
        pipeline.add_task(make_task("t1", 1)).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.total_requirements, 2);
        assert_eq!(summary.total_designs, 1);
        assert_eq!(summary.total_tasks, 1);
    }

    // --- Init ---

    #[test]
    fn test_init_spec() {
        let mut pipeline = make_pipeline();
        assert!(pipeline.init_spec().is_ok());
    }

    #[test]
    fn test_double_init_fails() {
        let mut pipeline = make_pipeline();
        pipeline.init_spec().unwrap();
        let result = pipeline.init_spec();
        assert!(matches!(result, Err(SpecError::InitError(_))));
    }

    // --- Error display ---

    #[test]
    fn test_error_display() {
        let err = SpecError::InvalidEarsFormat("bad text".to_string());
        assert_eq!(format!("{err}"), "invalid EARS format: bad text");
    }

    #[test]
    fn test_pattern_display() {
        assert_eq!(format!("{}", EarsPattern::Ubiquitous), "ubiquitous");
        assert_eq!(format!("{}", EarsPattern::EventDriven), "event-driven");
    }

    // --- Default config ---

    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();
        assert_eq!(config.spec_dir, ".spec");
        assert!(config.auto_validate);
        assert!(config.track_progress);
    }

    // --- Full lifecycle ---

    #[test]
    fn test_full_lifecycle() {
        let mut pipeline = make_pipeline();
        pipeline.init_spec().unwrap();

        // Add requirements via EARS parsing
        let mut req1 = pipeline
            .parse_ears("The authentication service shall verify JWT tokens")
            .unwrap();
        req1.priority = Priority::Must;
        let req_id1 = pipeline.add_requirement(req1).unwrap();

        let mut req2 = pipeline
            .parse_ears("When a token expires, the authentication service shall return 401")
            .unwrap();
        req2.priority = Priority::Must;
        let req_id2 = pipeline.add_requirement(req2).unwrap();

        // Add designs
        let mut design = make_design("JWT Auth Module");
        design.interfaces.push(InterfaceSpec {
            name: "verify_token".to_string(),
            interface_type: InterfaceType::Api,
            description: "Verify JWT".to_string(),
            inputs: vec!["token: String".to_string()],
            outputs: vec!["Result<Claims, AuthError>".to_string()],
        });
        let des_id = pipeline.add_design(design).unwrap();

        // Link requirements to design
        pipeline.link_requirement_to_design(&req_id1, &des_id).unwrap();
        pipeline.link_requirement_to_design(&req_id2, &des_id).unwrap();

        // Add tasks
        let mut t1 = make_task("Implement JWT verification", 1);
        t1.effort = Effort::Medium;
        let task_id1 = pipeline.add_task(t1).unwrap();

        let mut t2 = make_task("Add 401 response for expired tokens", 2);
        t2.effort = Effort::Small;
        t2.dependencies = vec![task_id1.clone()];
        let task_id2 = pipeline.add_task(t2).unwrap();

        // Link design to tasks
        pipeline.link_design_to_task(&des_id, &task_id1).unwrap();
        pipeline.link_design_to_task(&des_id, &task_id2).unwrap();

        // Validate — should be valid
        let validation = pipeline.validate();
        assert!(validation.is_valid, "errors: {:?}", validation.errors);

        // Check next tasks — only task1 (task2 depends on it)
        let next = pipeline.get_next_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, task_id1);

        // Complete task1, check task2 is now available
        pipeline.update_task_status(&task_id1, TaskStatus::Done).unwrap();
        let next = pipeline.get_next_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, task_id2);

        // Complete task2, check summary
        pipeline.update_task_status(&task_id2, TaskStatus::Done).unwrap();
        let summary = pipeline.get_summary();
        assert_eq!(summary.phase, PipelinePhase::Verification);
        assert_eq!(summary.total_requirements, 2);
        assert_eq!(summary.total_designs, 1);
        assert_eq!(summary.total_tasks, 2);
        assert_eq!(summary.coverage.tasks_completed, 2);

        // Generate markdown outputs
        let req_md = pipeline.generate_requirements_md();
        assert!(req_md.contains("REQ-001"));
        assert!(req_md.contains("REQ-002"));

        let design_md = pipeline.generate_design_md();
        assert!(design_md.contains("JWT Auth Module"));

        let tasks_md = pipeline.generate_tasks_md();
        assert!(tasks_md.contains("[x]"));
        assert!(tasks_md.contains("TASK-001"));
        assert!(tasks_md.contains("TASK-002"));
    }
}
