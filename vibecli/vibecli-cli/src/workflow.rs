//! Code Complete workflow system.
//!
//! Guides application development through 8 stages inspired by
//! Steve McConnell's *Code Complete* (2nd Edition):
//!
//! 1. Requirements
//! 2. Architecture
//! 3. Design
//! 4. Construction Planning
//! 5. Coding
//! 6. Quality Assurance
//! 7. Integration & Testing
//! 8. Code Complete
//!
//! Workflows are stored as markdown files in `.vibecli/workflows/`.
//!
//! # Usage
//! - `/workflow new <name> <description>` — create a new workflow
//! - `/workflow list`                     — list all workflows
//! - `/workflow show <name>`              — display workflow with stage progress
//! - `/workflow advance <name>`           — advance to next stage
//! - `/workflow check <name> <item-id>`   — toggle checklist item in current stage
//! - `/workflow generate <name>`          — AI-generate checklist for current stage

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ── WorkflowStage ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowStage {
    Requirements,
    Architecture,
    Design,
    ConstructionPlanning,
    Coding,
    QualityAssurance,
    Integration,
    CodeComplete,
}

impl WorkflowStage {
    pub const ALL: [WorkflowStage; 8] = [
        WorkflowStage::Requirements,
        WorkflowStage::Architecture,
        WorkflowStage::Design,
        WorkflowStage::ConstructionPlanning,
        WorkflowStage::Coding,
        WorkflowStage::QualityAssurance,
        WorkflowStage::Integration,
        WorkflowStage::CodeComplete,
    ];

    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|s| s == self).unwrap_or(0)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Requirements => "Requirements",
            Self::Architecture => "Architecture",
            Self::Design => "Design",
            Self::ConstructionPlanning => "Construction Planning",
            Self::Coding => "Coding",
            Self::QualityAssurance => "Quality Assurance",
            Self::Integration => "Integration & Testing",
            Self::CodeComplete => "Code Complete",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s.trim() {
            "Requirements" => Some(Self::Requirements),
            "Architecture" => Some(Self::Architecture),
            "Design" => Some(Self::Design),
            "Construction Planning" => Some(Self::ConstructionPlanning),
            "Coding" => Some(Self::Coding),
            "Quality Assurance" => Some(Self::QualityAssurance),
            "Integration & Testing" => Some(Self::Integration),
            "Code Complete" => Some(Self::CodeComplete),
            _ => None,
        }
    }

    pub fn next(&self) -> Option<Self> {
        let idx = self.index();
        Self::ALL.get(idx + 1).copied()
    }
}

impl fmt::Display for WorkflowStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── StageStatus ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageStatus {
    NotStarted,
    InProgress,
    Complete,
    Skipped,
}

impl Default for StageStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl fmt::Display for StageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not-started"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Complete => write!(f, "complete"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl StageStatus {
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "in-progress" => Self::InProgress,
            "complete" => Self::Complete,
            "skipped" => Self::Skipped,
            _ => Self::NotStarted,
        }
    }
}

// ── ChecklistItem ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: u32,
    pub description: String,
    pub done: bool,
}

impl ChecklistItem {
    pub fn new(id: u32, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            done: false,
        }
    }
}

// ── StageData ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageData {
    pub stage: WorkflowStage,
    pub status: StageStatus,
    pub checklist: Vec<ChecklistItem>,
    pub body: String,
}

impl StageData {
    pub fn new(stage: WorkflowStage) -> Self {
        Self {
            stage,
            status: StageStatus::NotStarted,
            checklist: vec![],
            body: String::new(),
        }
    }

    pub fn completed_count(&self) -> usize {
        self.checklist.iter().filter(|c| c.done).count()
    }

    pub fn total_count(&self) -> usize {
        self.checklist.len()
    }

    #[allow(dead_code)]
    pub fn progress_pct(&self) -> f64 {
        if self.checklist.is_empty() {
            return 0.0;
        }
        (self.completed_count() as f64 / self.total_count() as f64) * 100.0
    }
}

// ── Workflow ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub current_stage: WorkflowStage,
    pub stages: Vec<StageData>,
    pub created_at: String,
    pub source: PathBuf,
}

impl Workflow {
    /// Create a new workflow with all 8 empty stages.
    pub fn new(name: impl Into<String>, description: impl Into<String>, source: PathBuf) -> Self {
        let now = chrono_lite_now();
        Self {
            name: name.into(),
            description: description.into(),
            current_stage: WorkflowStage::Requirements,
            stages: WorkflowStage::ALL.iter().map(|s| StageData::new(*s)).collect(),
            created_at: now,
            source,
        }
    }

    /// Overall progress across all stages (0..100).
    pub fn overall_progress(&self) -> f64 {
        let total: usize = self.stages.iter().map(|s| s.total_count()).sum();
        let done: usize = self.stages.iter().map(|s| s.completed_count()).sum();
        if total == 0 {
            return 0.0;
        }
        (done as f64 / total as f64) * 100.0
    }

    /// Get the current stage data.
    pub fn current_stage_data(&self) -> &StageData {
        let idx = self.current_stage.index();
        &self.stages[idx]
    }

    /// Get mutable reference to current stage data.
    #[allow(dead_code)]
    pub fn current_stage_data_mut(&mut self) -> &mut StageData {
        let idx = self.current_stage.index();
        &mut self.stages[idx]
    }

    /// Serialize workflow to file content (YAML front-matter + markdown body).
    pub fn to_file_content(&self) -> String {
        let mut out = String::new();

        // Front-matter
        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", self.name));
        out.push_str(&format!("description: {}\n", self.description));
        out.push_str(&format!("current_stage: {}\n", self.current_stage.index()));
        out.push_str(&format!("created_at: {}\n", self.created_at));
        out.push_str("---\n\n");

        // Each stage as a section
        for stage in &self.stages {
            out.push_str(&format!("## Stage: {}\n", stage.stage.label()));
            out.push_str(&format!("<!-- status: {} -->\n\n", stage.status));

            if !stage.body.is_empty() {
                out.push_str(&stage.body);
                out.push_str("\n\n");
            }

            if !stage.checklist.is_empty() {
                out.push_str("### Checklist\n\n");
                for item in &stage.checklist {
                    let check = if item.done { "x" } else { " " };
                    out.push_str(&format!("- [{}] **{}**: {}\n", check, item.id, item.description));
                }
                out.push('\n');
            }
        }

        out
    }
}

/// Simple timestamp without chrono dependency.
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

// ── WorkflowManager ─────────────────────────────────────────────────────────

pub struct WorkflowManager {
    workflows_dir: PathBuf,
}

impl WorkflowManager {
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            workflows_dir: workspace_root.join(".vibecli").join("workflows"),
        }
    }

    #[allow(dead_code)]
    pub fn new(workflows_dir: PathBuf) -> Self {
        Self { workflows_dir }
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.workflows_dir)?;
        Ok(())
    }

    /// List all workflow names.
    pub fn list(&self) -> Vec<String> {
        if !self.workflows_dir.is_dir() {
            return vec![];
        }
        let mut names: Vec<String> = WalkDir::new(&self.workflows_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path().extension().and_then(|x| x.to_str()) == Some("md")
            })
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        names.sort();
        names
    }

    /// Load a workflow by name.
    pub fn load(&self, name: &str) -> Result<Workflow> {
        let path = self.workflows_dir.join(format!("{}.md", name));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read workflow '{}': {}", name, e))?;
        Self::parse(&path, name, &raw)
    }

    /// Save a workflow to disk.
    pub fn save(&self, workflow: &Workflow) -> Result<()> {
        std::fs::create_dir_all(&self.workflows_dir)?;
        let path = self.workflows_dir.join(format!("{}.md", workflow.name));
        std::fs::write(&path, workflow.to_file_content())?;
        Ok(())
    }

    /// Create a new workflow with 8 empty stages.
    pub fn create(&self, name: &str, description: &str) -> Result<Workflow> {
        self.init()?;
        let source = self.workflows_dir.join(format!("{}.md", name));
        let mut workflow = Workflow::new(name, description, source);
        // Mark first stage as in-progress
        workflow.stages[0].status = StageStatus::InProgress;
        self.save(&workflow)?;
        Ok(workflow)
    }

    /// Advance the current stage and move to the next.
    pub fn advance_stage(&self, name: &str) -> Result<Workflow> {
        let mut workflow = self.load(name)?;
        let idx = workflow.current_stage.index();

        // Mark current as complete
        workflow.stages[idx].status = StageStatus::Complete;

        // Advance to next stage if possible
        if let Some(next) = workflow.current_stage.next() {
            workflow.current_stage = next;
            let next_idx = next.index();
            workflow.stages[next_idx].status = StageStatus::InProgress;
        }

        self.save(&workflow)?;
        Ok(workflow)
    }

    /// Toggle a checklist item in a specific stage.
    pub fn toggle_checklist_item(
        &self,
        name: &str,
        stage_index: usize,
        item_id: u32,
        done: bool,
    ) -> Result<Workflow> {
        let mut workflow = self.load(name)?;
        if stage_index >= workflow.stages.len() {
            anyhow::bail!("Invalid stage index: {}", stage_index);
        }
        let stage = &mut workflow.stages[stage_index];
        if let Some(item) = stage.checklist.iter_mut().find(|c| c.id == item_id) {
            item.done = done;
        } else {
            anyhow::bail!("Checklist item {} not found in stage {}", item_id, stage_index);
        }

        // Auto-update stage status
        if stage.checklist.iter().all(|c| c.done) && !stage.checklist.is_empty() {
            stage.status = StageStatus::Complete;
        } else if stage.checklist.iter().any(|c| c.done) {
            stage.status = StageStatus::InProgress;
        }

        self.save(&workflow)?;
        Ok(workflow)
    }

    /// Set checklist items for a stage (used after AI generation).
    pub fn set_stage_checklist(
        &self,
        name: &str,
        stage_index: usize,
        items: Vec<ChecklistItem>,
    ) -> Result<Workflow> {
        let mut workflow = self.load(name)?;
        if stage_index >= workflow.stages.len() {
            anyhow::bail!("Invalid stage index: {}", stage_index);
        }
        workflow.stages[stage_index].checklist = items;
        if workflow.stages[stage_index].status == StageStatus::NotStarted {
            workflow.stages[stage_index].status = StageStatus::InProgress;
        }
        self.save(&workflow)?;
        Ok(workflow)
    }

    /// Parse a workflow from raw file contents.
    fn parse(path: &Path, name: &str, raw: &str) -> Result<Workflow> {
        let mut description = String::new();
        let mut current_stage_idx: usize = 0;
        let mut created_at = String::new();
        let mut body = raw.to_string();

        // Parse front-matter
        if raw.starts_with("---") {
            let after_open = raw[3..].trim_start_matches('\n');
            if let Some(close_pos) = after_open.find("\n---") {
                let fm = &after_open[..close_pos];
                body = after_open[close_pos..]
                    .trim_start_matches("\n---")
                    .trim_start()
                    .to_string();
                for line in fm.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        let key = k.trim();
                        let val = v.trim().trim_matches('"').trim_matches('\'');
                        match key {
                            "description" => description = val.to_string(),
                            "current_stage" => {
                                current_stage_idx = val.parse::<usize>().unwrap_or(0);
                            }
                            "created_at" => created_at = val.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }

        let current_stage = WorkflowStage::ALL
            .get(current_stage_idx)
            .copied()
            .unwrap_or(WorkflowStage::Requirements);

        // Parse stages from ## Stage: <label> sections
        let mut stages: Vec<StageData> = WorkflowStage::ALL.iter().map(|s| StageData::new(*s)).collect();

        let mut current_section: Option<usize> = None;
        let mut section_lines: Vec<String> = vec![];

        for line in body.lines() {
            if line.starts_with("## Stage: ") {
                // Flush previous section
                if let Some(idx) = current_section {
                    flush_stage_section(&mut stages[idx], &section_lines);
                }
                section_lines.clear();

                let label = &line["## Stage: ".len()..];
                if let Some(stage) = WorkflowStage::from_label(label) {
                    current_section = Some(stage.index());
                } else {
                    current_section = None;
                }
            } else if current_section.is_some() {
                section_lines.push(line.to_string());
            }
        }
        // Flush last section
        if let Some(idx) = current_section {
            flush_stage_section(&mut stages[idx], &section_lines);
        }

        Ok(Workflow {
            name: name.to_string(),
            description,
            current_stage,
            stages,
            created_at,
            source: path.to_path_buf(),
        })
    }
}

/// Parse status comment and checklist items from stage section lines.
fn flush_stage_section(stage: &mut StageData, lines: &[String]) {
    let mut body_lines: Vec<String> = vec![];
    let mut in_checklist = false;

    for line in lines {
        let trimmed = line.trim();

        // Parse <!-- status: ... --> comment
        if trimmed.starts_with("<!-- status:") && trimmed.ends_with("-->") {
            let inner = &trimmed["<!-- status:".len()..trimmed.len() - 3].trim();
            stage.status = StageStatus::from_str(inner);
            continue;
        }

        // Detect checklist header
        if trimmed == "### Checklist" {
            in_checklist = true;
            continue;
        }

        // Parse checklist items
        if in_checklist && trimmed.starts_with("- [") && trimmed.len() > 5 {
            let done = trimmed.starts_with("- [x]");
            let rest = trimmed[5..].trim();
            let (id, desc) = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some(idx) = stripped.find("**:") {
                    let id_str = &stripped[..idx];
                    let desc = stripped[idx + 3..].trim();
                    (
                        id_str.parse::<u32>().unwrap_or(stage.checklist.len() as u32 + 1),
                        desc.to_string(),
                    )
                } else {
                    (stage.checklist.len() as u32 + 1, rest.to_string())
                }
            } else {
                (stage.checklist.len() as u32 + 1, rest.to_string())
            };
            stage.checklist.push(ChecklistItem {
                id,
                description: desc,
                done,
            });
            continue;
        }

        // Empty line after checklist ends the checklist section
        if in_checklist && trimmed.is_empty() && !stage.checklist.is_empty() {
            in_checklist = false;
            continue;
        }

        if !in_checklist {
            body_lines.push(line.clone());
        }
    }

    stage.body = body_lines.join("\n").trim().to_string();
}

// ── LLM Prompts ─────────────────────────────────────────────────────────────

/// Build an LLM prompt to generate a stage-appropriate checklist.
pub fn stage_checklist_prompt(stage: &WorkflowStage, project_desc: &str) -> String {
    let stage_guidance = match stage {
        WorkflowStage::Requirements => r#"Generate a requirements checklist. Include items for:
- Specific functional requirements (core features, inputs/outputs)
- Non-functional requirements (performance, security, scalability, usability)
- User stories in brief form
- Scope boundaries (what's in/out)
- Error handling requirements
- Data requirements and constraints"#,

        WorkflowStage::Architecture => r#"Generate an architecture checklist. Include items for:
- System decomposition into subsystems/packages
- Inter-component communication strategy
- Data storage approach (database, file, cache)
- Error handling and logging strategy
- Security architecture (auth, encryption, input validation)
- Build vs buy decisions for major components
- Scalability and deployment considerations
- Third-party dependencies selection"#,

        WorkflowStage::Design => r#"Generate a detailed design checklist. Include items for:
- Key classes/modules identification and responsibilities
- Interface/API design for major components
- Data structures and algorithms selection
- Design patterns to apply (and rationale)
- Coupling and cohesion review
- Edge cases and boundary conditions
- State management approach
- Concurrency/async design (if applicable)"#,

        WorkflowStage::ConstructionPlanning => r#"Generate a construction planning checklist. Include items for:
- Programming language and framework choices confirmed
- Coding standards and naming conventions documented
- Development environment and tooling setup
- Source control branching strategy
- Integration order (bottom-up, top-down, or sandwich)
- Build and CI/CD pipeline setup
- Task breakdown and estimation
- Risk identification and mitigation"#,

        WorkflowStage::Coding => r#"Generate a coding quality checklist. Include items for:
- Variable naming follows conventions (clear, unambiguous)
- Defensive programming (assertions, error handling, bounds checks)
- No magic numbers (constants extracted and named)
- Functions/methods are short and do one thing
- Code duplication minimized (DRY principle)
- Control structures are straightforward (no deep nesting)
- Comments explain WHY not WHAT
- Input validation at system boundaries"#,

        WorkflowStage::QualityAssurance => r#"Generate a quality assurance checklist. Include items for:
- Code review completed (peer or AI-assisted)
- Unit tests written for core logic (coverage target met)
- Integration tests for component interactions
- Static analysis / linter passes with no warnings
- Security scan (no OWASP Top 10 vulnerabilities)
- Performance profiling done (no obvious bottlenecks)
- Error handling tested (invalid inputs, network failures)
- Accessibility review (if UI exists)"#,

        WorkflowStage::Integration => r#"Generate an integration and testing checklist. Include items for:
- All modules integrated and communicating correctly
- End-to-end tests pass for critical user flows
- Regression tests pass (no broken existing functionality)
- Performance under load tested
- Cross-platform/browser testing (if applicable)
- Database migration tested (if applicable)
- API contract validation (if applicable)
- Logging and monitoring verified"#,

        WorkflowStage::CodeComplete => r#"Generate a code-complete checklist. Include items for:
- All features implemented per requirements
- README and setup instructions updated
- API documentation generated/updated
- CHANGELOG updated with release notes
- License file present and correct
- No TODO/FIXME/HACK markers left in code
- Configuration externalized (no hardcoded secrets)
- Release version tagged in source control
- Deployment runbook documented
- Post-launch monitoring plan in place"#,
    };

    format!(
        r#"You are a software construction expert following Steve McConnell's Code Complete methodology.

Generate a checklist for the **{stage}** stage of this project:

Project: {project_desc}

{stage_guidance}

Output ONLY a numbered list of checklist items, one per line, like:
1. Description of first item
2. Description of second item
...

Generate 8-12 specific, actionable items tailored to this project. Be concise but clear."#,
        stage = stage.label(),
        project_desc = project_desc,
        stage_guidance = stage_guidance,
    )
}

/// Parse LLM output into checklist items.
pub fn parse_checklist_response(response: &str) -> Vec<ChecklistItem> {
    let mut items = vec![];
    let mut next_id = 1u32;

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Match patterns like "1. item", "1) item", "- item", "* item"
        let desc = if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            rest.trim().to_string()
        } else {
            // Try "N. " or "N) " pattern
            let mut chars = trimmed.chars().peekable();
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if !num_str.is_empty() {
                if let Some(&c) = chars.peek() {
                    if c == '.' || c == ')' {
                        chars.next(); // skip . or )
                        let rest: String = chars.collect();
                        rest.trim().to_string()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            }
        };

        if !desc.is_empty() {
            items.push(ChecklistItem::new(next_id, desc));
            next_id += 1;
        }
    }

    items
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        let workflow = mgr.create("todo_app", "Build a simple todo application").unwrap();
        assert_eq!(workflow.name, "todo_app");
        assert_eq!(workflow.current_stage, WorkflowStage::Requirements);
        assert_eq!(workflow.stages.len(), 8);
        assert_eq!(workflow.stages[0].status, StageStatus::InProgress);

        let loaded = mgr.load("todo_app").unwrap();
        assert_eq!(loaded.name, "todo_app");
        assert_eq!(loaded.description, "Build a simple todo application");
        assert_eq!(loaded.stages.len(), 8);
    }

    #[test]
    fn advance_stage_moves_forward() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        mgr.create("app", "Test app").unwrap();
        let workflow = mgr.advance_stage("app").unwrap();
        assert_eq!(workflow.current_stage, WorkflowStage::Architecture);
        assert_eq!(workflow.stages[0].status, StageStatus::Complete);
        assert_eq!(workflow.stages[1].status, StageStatus::InProgress);
    }

    #[test]
    fn advance_through_all_stages() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        mgr.create("app", "Test").unwrap();
        for _ in 0..7 {
            mgr.advance_stage("app").unwrap();
        }
        let workflow = mgr.load("app").unwrap();
        assert_eq!(workflow.current_stage, WorkflowStage::CodeComplete);
        // All stages before should be complete
        for i in 0..7 {
            assert_eq!(workflow.stages[i].status, StageStatus::Complete);
        }
    }

    #[test]
    fn toggle_checklist_item() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        mgr.create("app", "Test").unwrap();
        mgr.set_stage_checklist(
            "app",
            0,
            vec![
                ChecklistItem::new(1, "Define user stories"),
                ChecklistItem::new(2, "Set scope boundaries"),
            ],
        )
        .unwrap();

        let workflow = mgr.toggle_checklist_item("app", 0, 1, true).unwrap();
        assert!(workflow.stages[0].checklist[0].done);
        assert!(!workflow.stages[0].checklist[1].done);
        assert_eq!(workflow.stages[0].status, StageStatus::InProgress);

        let workflow = mgr.toggle_checklist_item("app", 0, 2, true).unwrap();
        assert_eq!(workflow.stages[0].status, StageStatus::Complete);
    }

    #[test]
    fn list_workflows() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        mgr.create("alpha", "First").unwrap();
        mgr.create("beta", "Second").unwrap();

        let names = mgr.list();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn overall_progress() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        mgr.create("app", "Test").unwrap();
        mgr.set_stage_checklist(
            "app",
            0,
            vec![
                ChecklistItem::new(1, "Item 1"),
                ChecklistItem::new(2, "Item 2"),
            ],
        )
        .unwrap();
        mgr.set_stage_checklist(
            "app",
            1,
            vec![
                ChecklistItem::new(1, "Arch 1"),
                ChecklistItem::new(2, "Arch 2"),
            ],
        )
        .unwrap();

        // Complete 1 of 4 total items
        mgr.toggle_checklist_item("app", 0, 1, true).unwrap();
        let workflow = mgr.load("app").unwrap();
        assert!((workflow.overall_progress() - 25.0).abs() < 0.1);
    }

    #[test]
    fn parse_checklist_response_numbered() {
        let response = "1. Define user stories for core flows\n2. Identify non-functional requirements\n3. Set scope boundaries\n";
        let items = parse_checklist_response(response);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, 1);
        assert_eq!(items[0].description, "Define user stories for core flows");
        assert_eq!(items[2].id, 3);
    }

    #[test]
    fn parse_checklist_response_mixed() {
        let response = "1) First item\n- Second item\n* Third item\n\nSome non-list text\n4. Fourth item\n";
        let items = parse_checklist_response(response);
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn stage_labels_roundtrip() {
        for stage in &WorkflowStage::ALL {
            let label = stage.label();
            let parsed = WorkflowStage::from_label(label);
            assert_eq!(parsed, Some(*stage), "Failed roundtrip for {}", label);
        }
    }

    #[test]
    fn stage_next() {
        assert_eq!(
            WorkflowStage::Requirements.next(),
            Some(WorkflowStage::Architecture)
        );
        assert_eq!(WorkflowStage::CodeComplete.next(), None);
    }

    #[test]
    fn checklist_with_body_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkflowManager::new(tmp.path().to_path_buf());

        let mut workflow = mgr.create("app", "Test app").unwrap();
        workflow.stages[0].body = "Some design notes here.".to_string();
        workflow.stages[0].checklist = vec![
            ChecklistItem { id: 1, description: "Item one".to_string(), done: true },
            ChecklistItem { id: 2, description: "Item two".to_string(), done: false },
        ];
        mgr.save(&workflow).unwrap();

        let loaded = mgr.load("app").unwrap();
        assert_eq!(loaded.stages[0].checklist.len(), 2);
        assert!(loaded.stages[0].checklist[0].done);
        assert!(!loaded.stages[0].checklist[1].done);
        assert!(loaded.stages[0].body.contains("design notes"));
    }
}
