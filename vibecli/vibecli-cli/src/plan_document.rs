//! Plan-as-document with feedback — markdown plan view with inline comments
//! for human feedback before execution.

use std::fmt;
use std::time::SystemTime;

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PlanStatus {
    Draft,
    InReview,
    Approved,
    Rejected,
    Executing,
    Completed,
    Abandoned,
}

impl fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::InReview => write!(f, "in-review"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Pending,
    Approved,
    Rejected,
    Modified,
    Executing,
    Completed,
    Skipped,
    Failed(String),
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Modified => write!(f, "modified"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::Skipped => write!(f, "skipped"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommentType {
    Approval,
    Rejection,
    Question,
    Suggestion,
    Note,
}

impl fmt::Display for CommentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approval => write!(f, "approval"),
            Self::Rejection => write!(f, "rejection"),
            Self::Question => write!(f, "question"),
            Self::Suggestion => write!(f, "suggestion"),
            Self::Note => write!(f, "note"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackAction {
    Approve,
    Reject,
    RequestChanges,
    AskQuestion,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Rename { from: String },
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Modify => write!(f, "modify"),
            Self::Delete => write!(f, "delete"),
            Self::Rename { from } => write!(f, "rename from {}", from),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PlanDocument {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    pub steps: Vec<PlanStep>,
    pub comments: Vec<PlanComment>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub author: String,
    pub reviewer: Option<String>,
    pub version: u32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: String,
    pub order: usize,
    pub title: String,
    pub description: String,
    pub status: StepStatus,
    pub file_changes: Vec<FileChange>,
    pub estimated_lines: usize,
    pub dependencies: Vec<String>,
    pub comments: Vec<PlanComment>,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub description: String,
    pub estimated_diff_lines: usize,
}

#[derive(Debug, Clone)]
pub struct PlanComment {
    pub id: String,
    pub author: String,
    pub comment_type: CommentType,
    pub body: String,
    pub step_id: Option<String>,
    pub line_ref: Option<usize>,
    pub timestamp: SystemTime,
    pub resolved: bool,
}

#[derive(Debug)]
pub struct PlanManager {
    pub plans: Vec<PlanDocument>,
    pub active_plan: Option<String>,
    next_plan_id: u64,
    next_step_id: u64,
    next_comment_id: u64,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn gen_id(prefix: &str, counter: u64) -> String {
    format!("{}-{}", prefix, counter)
}

// ─── PlanComment impl ───────────────────────────────────────────────────────

impl PlanComment {
    pub fn new(
        id: &str,
        author: &str,
        comment_type: CommentType,
        body: &str,
        step_id: Option<String>,
        line_ref: Option<usize>,
    ) -> Self {
        Self {
            id: id.to_string(),
            author: author.to_string(),
            comment_type,
            body: body.to_string(),
            step_id,
            line_ref,
            timestamp: SystemTime::now(),
            resolved: false,
        }
    }
}

// ─── PlanStep impl ──────────────────────────────────────────────────────────

impl PlanStep {
    pub fn new(order: usize, title: &str, description: &str) -> Self {
        Self {
            id: String::new(), // set by PlanDocument::add_step
            order,
            title: title.to_string(),
            description: description.to_string(),
            status: StepStatus::Pending,
            file_changes: Vec::new(),
            estimated_lines: 0,
            dependencies: Vec::new(),
            comments: Vec::new(),
        }
    }

    pub fn add_file_change(&mut self, change: FileChange) {
        self.estimated_lines += change.estimated_diff_lines;
        self.file_changes.push(change);
    }

    pub fn approve(&mut self) {
        self.status = StepStatus::Approved;
    }

    pub fn reject(&mut self, reason: &str) {
        self.status = StepStatus::Failed(reason.to_string());
    }

    pub fn skip(&mut self) {
        self.status = StepStatus::Skipped;
    }

    pub fn complete(&mut self) {
        self.status = StepStatus::Completed;
    }

    /// Status badge suitable for markdown rendering.
    fn status_badge(&self) -> &str {
        match &self.status {
            StepStatus::Pending => "[ ]",
            StepStatus::Approved => "[approved]",
            StepStatus::Rejected => "[rejected]",
            StepStatus::Modified => "[modified]",
            StepStatus::Executing => "[running]",
            StepStatus::Completed => "[x]",
            StepStatus::Skipped => "[skipped]",
            StepStatus::Failed(_) => "[failed]",
        }
    }
}

// ─── PlanDocument impl ──────────────────────────────────────────────────────

impl PlanDocument {
    pub fn new(title: &str, description: &str, author: &str) -> Self {
        let now = SystemTime::now();
        Self {
            id: String::new(), // set by PlanManager
            title: title.to_string(),
            description: description.to_string(),
            status: PlanStatus::Draft,
            steps: Vec::new(),
            comments: Vec::new(),
            created_at: now,
            updated_at: now,
            author: author.to_string(),
            reviewer: None,
            version: 1,
            tags: Vec::new(),
        }
    }

    /// Add a step to the plan; returns a mutable reference to the inserted step.
    pub fn add_step(&mut self, mut step: PlanStep, step_id: &str) -> &mut PlanStep {
        step.id = step_id.to_string();
        self.steps.push(step);
        self.updated_at = SystemTime::now();
        self.steps.last_mut().expect("just pushed a step")
    }

    /// Remove a step by ID. Returns true if found and removed.
    pub fn remove_step(&mut self, id: &str) -> bool {
        let before = self.steps.len();
        self.steps.retain(|s| s.id != id);
        let removed = self.steps.len() < before;
        if removed {
            self.updated_at = SystemTime::now();
        }
        removed
    }

    /// Re-sort steps by their `order` field.
    pub fn reorder_steps(&mut self) {
        self.steps.sort_by_key(|s| s.order);
        self.updated_at = SystemTime::now();
    }

    /// Add a plan-level comment.
    pub fn add_comment(&mut self, comment: PlanComment) {
        self.comments.push(comment);
        self.updated_at = SystemTime::now();
    }

    /// Submit the plan for review.
    pub fn submit_for_review(&mut self, reviewer: &str) {
        self.status = PlanStatus::InReview;
        self.reviewer = Some(reviewer.to_string());
        self.updated_at = SystemTime::now();
    }

    /// Approve the plan.
    pub fn approve(&mut self, reviewer: &str) {
        self.status = PlanStatus::Approved;
        self.reviewer = Some(reviewer.to_string());
        self.updated_at = SystemTime::now();
    }

    /// Reject the plan with a reason (added as a comment).
    pub fn reject(&mut self, reviewer: &str, reason: &str) {
        self.status = PlanStatus::Rejected;
        self.reviewer = Some(reviewer.to_string());
        let comment = PlanComment::new(
            &format!("reject-{}", self.version),
            reviewer,
            CommentType::Rejection,
            reason,
            None,
            None,
        );
        self.comments.push(comment);
        self.updated_at = SystemTime::now();
    }

    /// Export the full plan as a markdown document.
    pub fn to_markdown(&self) -> String {
        let mut md = String::with_capacity(1024);
        md.push_str(&format!("# {}\n\n", self.title));
        md.push_str(&format!(
            "**Status:** {} | **Version:** {} | **Author:** {}\n\n",
            self.status, self.version, self.author
        ));
        if !self.description.is_empty() {
            md.push_str(&format!("{}\n\n", self.description));
        }
        if !self.tags.is_empty() {
            md.push_str(&format!("**Tags:** {}\n\n", self.tags.join(", ")));
        }

        // Plan-level comments
        for c in &self.comments {
            let resolved_tag = if c.resolved { " [RESOLVED]" } else { "" };
            md.push_str(&format!(
                "> **{}** ({}){}: {}\n\n",
                c.author, c.comment_type, resolved_tag, c.body
            ));
        }

        md.push_str("## Steps\n\n");
        for step in &self.steps {
            md.push_str(&format!(
                "### {} {}. {}\n\n",
                step.status_badge(),
                step.order,
                step.title
            ));
            md.push_str(&format!("{}\n\n", step.description));

            if !step.file_changes.is_empty() {
                md.push_str("**File changes:**\n");
                for fc in &step.file_changes {
                    md.push_str(&format!(
                        "- `{}` ({}, ~{} lines) — {}\n",
                        fc.path, fc.change_type, fc.estimated_diff_lines, fc.description
                    ));
                }
                md.push('\n');
            }

            if !step.dependencies.is_empty() {
                md.push_str(&format!(
                    "**Depends on:** {}\n\n",
                    step.dependencies.join(", ")
                ));
            }

            for c in &step.comments {
                let resolved_tag = if c.resolved { " [RESOLVED]" } else { "" };
                md.push_str(&format!(
                    "> **{}** ({}){}: {}\n\n",
                    c.author, c.comment_type, resolved_tag, c.body
                ));
            }
        }

        md
    }

    /// Parse a markdown string back into a PlanDocument.
    /// Supports the format produced by `to_markdown()`.
    pub fn from_markdown(md: &str) -> Result<Self, String> {
        let lines: Vec<&str> = md.lines().collect();
        if lines.is_empty() {
            return Err("empty markdown".to_string());
        }

        // Parse title from first line: "# Title"
        let title = lines
            .first()
            .and_then(|l| l.strip_prefix("# "))
            .ok_or("missing title line")?
            .to_string();

        // Parse status/version/author line
        let mut status = PlanStatus::Draft;
        let mut version: u32 = 1;
        let mut author = String::from("unknown");

        for line in &lines {
            if line.starts_with("**Status:**") {
                // Parse "**Status:** draft | **Version:** 1 | **Author:** agent"
                let parts: Vec<&str> = line.split('|').collect();
                for part in &parts {
                    let trimmed = part.trim();
                    if let Some(s) = trimmed.strip_prefix("**Status:**") {
                        status = match s.trim() {
                            "draft" => PlanStatus::Draft,
                            "in-review" => PlanStatus::InReview,
                            "approved" => PlanStatus::Approved,
                            "rejected" => PlanStatus::Rejected,
                            "executing" => PlanStatus::Executing,
                            "completed" => PlanStatus::Completed,
                            "abandoned" => PlanStatus::Abandoned,
                            _ => PlanStatus::Draft,
                        };
                    }
                    if let Some(v) = trimmed.strip_prefix("**Version:**") {
                        version = v.trim().parse().unwrap_or(1);
                    }
                    if let Some(a) = trimmed.strip_prefix("**Author:**") {
                        author = a.trim().to_string();
                    }
                }
                break;
            }
        }

        let mut doc = PlanDocument::new(&title, "", &author);
        doc.status = status;
        doc.version = version;
        Ok(doc)
    }

    /// Return all unresolved comments (plan-level + step-level).
    pub fn unresolved_comments(&self) -> Vec<&PlanComment> {
        let mut result: Vec<&PlanComment> = self
            .comments
            .iter()
            .filter(|c| !c.resolved)
            .collect();
        for step in &self.steps {
            for c in &step.comments {
                if !c.resolved {
                    result.push(c);
                }
            }
        }
        result
    }

    /// Find a step by ID.
    pub fn step_by_id(&self, id: &str) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Find a mutable step by ID.
    pub fn step_by_id_mut(&mut self, id: &str) -> Option<&mut PlanStep> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    /// Percentage of steps that are completed or skipped.
    pub fn progress_percentage(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        let done = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped)
            .count();
        (done as f64 / self.steps.len() as f64) * 100.0
    }

    /// Total estimated lines across all steps.
    pub fn total_estimated_lines(&self) -> usize {
        self.steps.iter().map(|s| s.estimated_lines).sum()
    }

    /// Apply a feedback action, adding a corresponding comment.
    pub fn apply_feedback(&mut self, action: FeedbackAction, comment: PlanComment) {
        match action {
            FeedbackAction::Approve => {
                self.status = PlanStatus::Approved;
            }
            FeedbackAction::Reject => {
                self.status = PlanStatus::Rejected;
            }
            FeedbackAction::RequestChanges => {
                self.status = PlanStatus::Draft;
                self.version += 1;
            }
            FeedbackAction::AskQuestion => {
                // status unchanged, just add comment
            }
        }
        self.comments.push(comment);
        self.updated_at = SystemTime::now();
    }
}

// ─── PlanManager impl ───────────────────────────────────────────────────────

impl PlanManager {
    pub fn new() -> Self {
        Self {
            plans: Vec::new(),
            active_plan: None,
            next_plan_id: 0,
            next_step_id: 0,
            next_comment_id: 0,
        }
    }

    /// Create a new plan and return a reference to it.
    pub fn create_plan(
        &mut self,
        title: &str,
        desc: &str,
        author: &str,
    ) -> &PlanDocument {
        self.next_plan_id += 1;
        let mut doc = PlanDocument::new(title, desc, author);
        doc.id = gen_id("plan", self.next_plan_id);
        self.plans.push(doc);
        self.plans.last().expect("just pushed a plan")
    }

    /// Look up a plan by ID.
    pub fn get_plan(&self, id: &str) -> Option<&PlanDocument> {
        self.plans.iter().find(|p| p.id == id)
    }

    /// Look up a mutable plan by ID.
    pub fn get_plan_mut(&mut self, id: &str) -> Option<&mut PlanDocument> {
        self.plans.iter_mut().find(|p| p.id == id)
    }

    /// List all plans as (id, title, status) tuples.
    pub fn list_plans(&self) -> Vec<(&str, &str, &PlanStatus)> {
        self.plans
            .iter()
            .map(|p| (p.id.as_str(), p.title.as_str(), &p.status))
            .collect()
    }

    /// Get the active plan.
    pub fn active_plan(&self) -> Option<&PlanDocument> {
        self.active_plan
            .as_ref()
            .and_then(|id| self.plans.iter().find(|p| &p.id == id))
    }

    /// Set the active plan by ID. Returns false if the ID was not found.
    pub fn set_active(&mut self, id: &str) -> bool {
        if self.plans.iter().any(|p| p.id == id) {
            self.active_plan = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Generate the next step ID.
    pub fn next_step_id(&mut self) -> String {
        self.next_step_id += 1;
        gen_id("step", self.next_step_id)
    }

    /// Generate the next comment ID.
    pub fn next_comment_id(&mut self) -> String {
        self.next_comment_id += 1;
        gen_id("comment", self.next_comment_id)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── PlanDocument tests ──

    #[test]
    fn test_plan_document_new_defaults() {
        let doc = PlanDocument::new("Refactor auth", "Rewrite auth module", "agent");
        assert_eq!(doc.title, "Refactor auth");
        assert_eq!(doc.description, "Rewrite auth module");
        assert_eq!(doc.author, "agent");
        assert_eq!(doc.status, PlanStatus::Draft);
        assert_eq!(doc.version, 1);
        assert!(doc.steps.is_empty());
        assert!(doc.comments.is_empty());
        assert!(doc.reviewer.is_none());
        assert!(doc.tags.is_empty());
    }

    #[test]
    fn test_plan_add_step() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let step = PlanStep::new(1, "Step 1", "Do the thing");
        let s = doc.add_step(step, "step-1");
        assert_eq!(s.id, "step-1");
        assert_eq!(s.order, 1);
        assert_eq!(doc.steps.len(), 1);
    }

    #[test]
    fn test_plan_remove_step() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_step(PlanStep::new(1, "Step 1", "first"), "s1");
        doc.add_step(PlanStep::new(2, "Step 2", "second"), "s2");
        assert!(doc.remove_step("s1"));
        assert_eq!(doc.steps.len(), 1);
        assert_eq!(doc.steps[0].id, "s2");
    }

    #[test]
    fn test_plan_remove_step_not_found() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        assert!(!doc.remove_step("nonexistent"));
    }

    #[test]
    fn test_plan_reorder_steps() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_step(PlanStep::new(3, "Third", "c"), "s3");
        doc.add_step(PlanStep::new(1, "First", "a"), "s1");
        doc.add_step(PlanStep::new(2, "Second", "b"), "s2");
        doc.reorder_steps();
        assert_eq!(doc.steps[0].order, 1);
        assert_eq!(doc.steps[1].order, 2);
        assert_eq!(doc.steps[2].order, 3);
    }

    #[test]
    fn test_plan_add_comment() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let c = PlanComment::new("c1", "bob", CommentType::Question, "Why?", None, None);
        doc.add_comment(c);
        assert_eq!(doc.comments.len(), 1);
    }

    #[test]
    fn test_plan_submit_for_review() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.submit_for_review("bob");
        assert_eq!(doc.status, PlanStatus::InReview);
        assert_eq!(doc.reviewer.as_deref(), Some("bob"));
    }

    #[test]
    fn test_plan_approve() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.approve("bob");
        assert_eq!(doc.status, PlanStatus::Approved);
        assert_eq!(doc.reviewer.as_deref(), Some("bob"));
    }

    #[test]
    fn test_plan_reject() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.reject("bob", "Not good enough");
        assert_eq!(doc.status, PlanStatus::Rejected);
        assert_eq!(doc.comments.len(), 1);
        assert_eq!(doc.comments[0].comment_type, CommentType::Rejection);
        assert_eq!(doc.comments[0].body, "Not good enough");
    }

    #[test]
    fn test_plan_to_markdown_has_title() {
        let doc = PlanDocument::new("My Plan", "A description", "agent");
        let md = doc.to_markdown();
        assert!(md.contains("# My Plan"));
    }

    #[test]
    fn test_plan_to_markdown_has_status() {
        let doc = PlanDocument::new("My Plan", "desc", "agent");
        let md = doc.to_markdown();
        assert!(md.contains("**Status:** draft"));
    }

    #[test]
    fn test_plan_to_markdown_with_steps() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_step(PlanStep::new(1, "Step One", "Do something"), "s1");
        let md = doc.to_markdown();
        assert!(md.contains("Step One"));
        assert!(md.contains("## Steps"));
    }

    #[test]
    fn test_plan_to_markdown_with_file_changes() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut step = PlanStep::new(1, "Refactor", "Rewrite module");
        step.add_file_change(FileChange {
            path: "src/lib.rs".to_string(),
            change_type: ChangeType::Modify,
            description: "Add new function".to_string(),
            estimated_diff_lines: 20,
        });
        doc.add_step(step, "s1");
        let md = doc.to_markdown();
        assert!(md.contains("`src/lib.rs`"));
        assert!(md.contains("modify"));
    }

    #[test]
    fn test_plan_to_markdown_with_comments() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_comment(PlanComment::new(
            "c1",
            "bob",
            CommentType::Approval,
            "Looks good",
            None,
            None,
        ));
        let md = doc.to_markdown();
        assert!(md.contains("Looks good"));
        assert!(md.contains("**bob**"));
    }

    #[test]
    fn test_plan_from_markdown_basic() {
        let md = "# Test Plan\n\n**Status:** draft | **Version:** 3 | **Author:** alice\n";
        let doc = PlanDocument::from_markdown(md).unwrap();
        assert_eq!(doc.title, "Test Plan");
        assert_eq!(doc.status, PlanStatus::Draft);
        assert_eq!(doc.version, 3);
        assert_eq!(doc.author, "alice");
    }

    #[test]
    fn test_plan_from_markdown_approved() {
        let md = "# Approved Plan\n\n**Status:** approved | **Version:** 1 | **Author:** bot\n";
        let doc = PlanDocument::from_markdown(md).unwrap();
        assert_eq!(doc.status, PlanStatus::Approved);
    }

    #[test]
    fn test_plan_from_markdown_empty_error() {
        let result = PlanDocument::from_markdown("");
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_from_markdown_no_title_error() {
        let result = PlanDocument::from_markdown("not a title");
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_unresolved_comments_plan_level() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_comment(PlanComment::new(
            "c1",
            "bob",
            CommentType::Question,
            "Why?",
            None,
            None,
        ));
        assert_eq!(doc.unresolved_comments().len(), 1);
    }

    #[test]
    fn test_plan_unresolved_comments_resolved() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut c = PlanComment::new("c1", "bob", CommentType::Note, "ok", None, None);
        c.resolved = true;
        doc.add_comment(c);
        assert_eq!(doc.unresolved_comments().len(), 0);
    }

    #[test]
    fn test_plan_unresolved_comments_step_level() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut step = PlanStep::new(1, "Step", "do it");
        step.comments.push(PlanComment::new(
            "c1",
            "carol",
            CommentType::Suggestion,
            "try X",
            Some("s1".to_string()),
            None,
        ));
        doc.add_step(step, "s1");
        assert_eq!(doc.unresolved_comments().len(), 1);
    }

    #[test]
    fn test_plan_step_by_id() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.add_step(PlanStep::new(1, "First", "a"), "s1");
        assert!(doc.step_by_id("s1").is_some());
        assert!(doc.step_by_id("s99").is_none());
    }

    #[test]
    fn test_plan_progress_percentage_empty() {
        let doc = PlanDocument::new("Plan", "desc", "agent");
        assert_eq!(doc.progress_percentage(), 0.0);
    }

    #[test]
    fn test_plan_progress_percentage_half() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut step1 = PlanStep::new(1, "S1", "a");
        step1.complete();
        doc.add_step(step1, "s1");
        doc.add_step(PlanStep::new(2, "S2", "b"), "s2");
        assert!((doc.progress_percentage() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_plan_progress_percentage_with_skipped() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut step1 = PlanStep::new(1, "S1", "a");
        step1.skip();
        doc.add_step(step1, "s1");
        assert!((doc.progress_percentage() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_plan_total_estimated_lines() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let mut step = PlanStep::new(1, "S1", "a");
        step.add_file_change(FileChange {
            path: "a.rs".to_string(),
            change_type: ChangeType::Create,
            description: "new file".to_string(),
            estimated_diff_lines: 50,
        });
        step.add_file_change(FileChange {
            path: "b.rs".to_string(),
            change_type: ChangeType::Modify,
            description: "change".to_string(),
            estimated_diff_lines: 30,
        });
        doc.add_step(step, "s1");
        assert_eq!(doc.total_estimated_lines(), 80);
    }

    #[test]
    fn test_plan_apply_feedback_approve() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let c = PlanComment::new("c1", "bob", CommentType::Approval, "LGTM", None, None);
        doc.apply_feedback(FeedbackAction::Approve, c);
        assert_eq!(doc.status, PlanStatus::Approved);
        assert_eq!(doc.comments.len(), 1);
    }

    #[test]
    fn test_plan_apply_feedback_reject() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let c = PlanComment::new("c1", "bob", CommentType::Rejection, "nope", None, None);
        doc.apply_feedback(FeedbackAction::Reject, c);
        assert_eq!(doc.status, PlanStatus::Rejected);
    }

    #[test]
    fn test_plan_apply_feedback_request_changes() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        let c = PlanComment::new("c1", "bob", CommentType::Suggestion, "change X", None, None);
        doc.apply_feedback(FeedbackAction::RequestChanges, c);
        assert_eq!(doc.status, PlanStatus::Draft);
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_plan_apply_feedback_ask_question() {
        let mut doc = PlanDocument::new("Plan", "desc", "agent");
        doc.status = PlanStatus::InReview;
        let c = PlanComment::new("c1", "bob", CommentType::Question, "why?", None, None);
        doc.apply_feedback(FeedbackAction::AskQuestion, c);
        assert_eq!(doc.status, PlanStatus::InReview); // unchanged
    }

    // ── PlanStep tests ──

    #[test]
    fn test_step_new() {
        let s = PlanStep::new(1, "Setup", "Initialize project");
        assert_eq!(s.order, 1);
        assert_eq!(s.title, "Setup");
        assert_eq!(s.status, StepStatus::Pending);
        assert!(s.file_changes.is_empty());
    }

    #[test]
    fn test_step_add_file_change_updates_estimated_lines() {
        let mut s = PlanStep::new(1, "S", "d");
        s.add_file_change(FileChange {
            path: "x.rs".to_string(),
            change_type: ChangeType::Create,
            description: "new".to_string(),
            estimated_diff_lines: 10,
        });
        assert_eq!(s.estimated_lines, 10);
        assert_eq!(s.file_changes.len(), 1);
    }

    #[test]
    fn test_step_approve() {
        let mut s = PlanStep::new(1, "S", "d");
        s.approve();
        assert_eq!(s.status, StepStatus::Approved);
    }

    #[test]
    fn test_step_reject() {
        let mut s = PlanStep::new(1, "S", "d");
        s.reject("bad approach");
        assert_eq!(s.status, StepStatus::Failed("bad approach".to_string()));
    }

    #[test]
    fn test_step_skip() {
        let mut s = PlanStep::new(1, "S", "d");
        s.skip();
        assert_eq!(s.status, StepStatus::Skipped);
    }

    #[test]
    fn test_step_complete() {
        let mut s = PlanStep::new(1, "S", "d");
        s.complete();
        assert_eq!(s.status, StepStatus::Completed);
    }

    // ── PlanManager tests ──

    #[test]
    fn test_manager_new() {
        let mgr = PlanManager::new();
        assert!(mgr.plans.is_empty());
        assert!(mgr.active_plan.is_none());
    }

    #[test]
    fn test_manager_create_plan() {
        let mut mgr = PlanManager::new();
        let plan = mgr.create_plan("P1", "description", "alice");
        assert_eq!(plan.title, "P1");
        assert_eq!(plan.id, "plan-1");
        assert_eq!(mgr.plans.len(), 1);
    }

    #[test]
    fn test_manager_get_plan() {
        let mut mgr = PlanManager::new();
        let id = mgr.create_plan("P1", "d", "alice").id.clone();
        assert!(mgr.get_plan(&id).is_some());
        assert!(mgr.get_plan("fake").is_none());
    }

    #[test]
    fn test_manager_list_plans() {
        let mut mgr = PlanManager::new();
        mgr.create_plan("P1", "d", "alice");
        mgr.create_plan("P2", "d", "bob");
        let list = mgr.list_plans();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].1, "P1");
        assert_eq!(list[1].1, "P2");
    }

    #[test]
    fn test_manager_active_plan_none() {
        let mgr = PlanManager::new();
        assert!(mgr.active_plan().is_none());
    }

    #[test]
    fn test_manager_set_active() {
        let mut mgr = PlanManager::new();
        let id = mgr.create_plan("P1", "d", "alice").id.clone();
        assert!(mgr.set_active(&id));
        assert!(mgr.active_plan().is_some());
        assert_eq!(mgr.active_plan().unwrap().title, "P1");
    }

    #[test]
    fn test_manager_set_active_not_found() {
        let mut mgr = PlanManager::new();
        assert!(!mgr.set_active("nonexistent"));
    }

    #[test]
    fn test_manager_next_step_id() {
        let mut mgr = PlanManager::new();
        assert_eq!(mgr.next_step_id(), "step-1");
        assert_eq!(mgr.next_step_id(), "step-2");
    }

    #[test]
    fn test_manager_next_comment_id() {
        let mut mgr = PlanManager::new();
        assert_eq!(mgr.next_comment_id(), "comment-1");
        assert_eq!(mgr.next_comment_id(), "comment-2");
    }

    // ── Display tests ──

    #[test]
    fn test_plan_status_display() {
        assert_eq!(format!("{}", PlanStatus::Draft), "draft");
        assert_eq!(format!("{}", PlanStatus::InReview), "in-review");
        assert_eq!(format!("{}", PlanStatus::Approved), "approved");
        assert_eq!(format!("{}", PlanStatus::Rejected), "rejected");
        assert_eq!(format!("{}", PlanStatus::Executing), "executing");
        assert_eq!(format!("{}", PlanStatus::Completed), "completed");
        assert_eq!(format!("{}", PlanStatus::Abandoned), "abandoned");
    }

    #[test]
    fn test_step_status_display() {
        assert_eq!(format!("{}", StepStatus::Pending), "pending");
        assert_eq!(format!("{}", StepStatus::Completed), "completed");
        assert_eq!(
            format!("{}", StepStatus::Failed("oops".to_string())),
            "failed: oops"
        );
    }

    #[test]
    fn test_comment_type_display() {
        assert_eq!(format!("{}", CommentType::Approval), "approval");
        assert_eq!(format!("{}", CommentType::Rejection), "rejection");
        assert_eq!(format!("{}", CommentType::Question), "question");
        assert_eq!(format!("{}", CommentType::Suggestion), "suggestion");
        assert_eq!(format!("{}", CommentType::Note), "note");
    }

    #[test]
    fn test_change_type_display() {
        assert_eq!(format!("{}", ChangeType::Create), "create");
        assert_eq!(format!("{}", ChangeType::Modify), "modify");
        assert_eq!(format!("{}", ChangeType::Delete), "delete");
        assert_eq!(
            format!(
                "{}",
                ChangeType::Rename {
                    from: "old.rs".to_string()
                }
            ),
            "rename from old.rs"
        );
    }

    #[test]
    fn test_roundtrip_markdown() {
        let mut doc = PlanDocument::new("Roundtrip", "A test plan", "agent");
        doc.id = "plan-99".to_string();
        doc.version = 5;
        let md = doc.to_markdown();
        let parsed = PlanDocument::from_markdown(&md).unwrap();
        assert_eq!(parsed.title, "Roundtrip");
        assert_eq!(parsed.version, 5);
        assert_eq!(parsed.author, "agent");
    }

    #[test]
    fn test_plan_tags_in_markdown() {
        let mut doc = PlanDocument::new("Tagged", "desc", "agent");
        doc.tags = vec!["refactor".to_string(), "auth".to_string()];
        let md = doc.to_markdown();
        assert!(md.contains("**Tags:** refactor, auth"));
    }

    #[test]
    fn test_plan_step_dependencies_in_markdown() {
        let mut doc = PlanDocument::new("Plan", "d", "agent");
        let mut step = PlanStep::new(1, "S1", "desc");
        step.dependencies = vec!["step-0".to_string()];
        doc.add_step(step, "s1");
        let md = doc.to_markdown();
        assert!(md.contains("**Depends on:** step-0"));
    }
}
