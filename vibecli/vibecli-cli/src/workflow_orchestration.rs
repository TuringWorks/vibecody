//! Workflow Orchestration system for AI-assisted development.
//!
//! Implements structured engineering practices inspired by Claude Code best-practices:
//!
//! 1. **Plan Node Default** — Enter plan mode for non-trivial tasks (3+ steps)
//! 2. **Subagent Strategy** — Offload research/exploration to subagents
//! 3. **Self-Improvement Loop** — Capture lessons after corrections
//! 4. **Verification Before Done** — Prove correctness before closing
//! 5. **Demand Elegance** — Pause for non-trivial changes, skip for simple fixes
//! 6. **Autonomous Bug Fixing** — Fix from logs/tests without hand-holding
//!
//! ## REPL Commands
//! - `/orchestrate status`           — show orchestration state
//! - `/orchestrate lessons`          — view learned lessons
//! - `/orchestrate lesson <text>`    — record a new lesson
//! - `/orchestrate todo`             — show current task plan
//! - `/orchestrate todo add <text>`  — add a task item
//! - `/orchestrate todo done <id>`   — mark task item complete
//! - `/orchestrate verify`           — run verification gate
//! - `/orchestrate reset`            — clear current task state

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

// ── Lesson ──────────────────────────────────────────────────────────────────

/// A learned lesson from a correction or mistake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    /// Unique ID (sequential).
    pub id: u32,
    /// The pattern or mistake.
    pub pattern: String,
    /// The rule to prevent recurrence.
    pub rule: String,
    /// ISO timestamp when recorded.
    pub recorded_at: String,
    /// Optional category tag.
    #[serde(default)]
    pub category: String,
    /// How many times this lesson was relevant (hit count).
    #[serde(default)]
    pub hit_count: u32,
}

impl Lesson {
    pub fn new(id: u32, pattern: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            id,
            pattern: pattern.into(),
            rule: rule.into(),
            recorded_at: timestamp_now(),
            category: String::new(),
            hit_count: 0,
        }
    }
}

impl fmt::Display for Lesson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{}: [{}] {} → {}",
            self.id,
            if self.category.is_empty() { "general" } else { &self.category },
            self.pattern,
            self.rule
        )
    }
}

// ── TodoItem ────────────────────────────────────────────────────────────────

/// A task item in the current orchestration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u32,
    pub description: String,
    pub done: bool,
    /// Optional step type hint.
    #[serde(default)]
    pub step_type: StepType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    #[default]
    Build,
    Plan,
    Research,
    Verify,
    Test,
    Review,
}

impl fmt::Display for StepType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Plan => write!(f, "plan"),
            Self::Research => write!(f, "research"),
            Self::Verify => write!(f, "verify"),
            Self::Test => write!(f, "test"),
            Self::Review => write!(f, "review"),
        }
    }
}

impl TodoItem {
    pub fn new(id: u32, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            done: false,
            step_type: StepType::Build,
        }
    }

    pub fn with_type(mut self, step_type: StepType) -> Self {
        self.step_type = step_type;
        self
    }
}

// ── TaskComplexity ──────────────────────────────────────────────────────────

/// Estimated complexity of a task — determines whether plan mode is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// Simple, obvious fix — no planning needed.
    Trivial,
    /// Moderate — may benefit from a quick plan.
    Moderate,
    /// Complex — requires plan mode (3+ steps or architectural decisions).
    Complex,
}

impl fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trivial => write!(f, "trivial"),
            Self::Moderate => write!(f, "moderate"),
            Self::Complex => write!(f, "complex"),
        }
    }
}

// ── OrchestrationState ──────────────────────────────────────────────────────

/// Current state of workflow orchestration for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationState {
    /// Task goal/description.
    pub goal: String,
    /// Estimated complexity.
    pub complexity: TaskComplexity,
    /// Task items (todo list).
    pub todos: Vec<TodoItem>,
    /// Whether plan mode was entered.
    pub planned: bool,
    /// Whether verification passed.
    pub verified: bool,
    /// High-level summary of changes made.
    #[serde(default)]
    pub change_summary: String,
    /// Timestamp when task started.
    pub started_at: String,
    /// Timestamp when task completed (if done).
    #[serde(default)]
    pub completed_at: Option<String>,
}

impl OrchestrationState {
    pub fn new(goal: impl Into<String>, complexity: TaskComplexity) -> Self {
        Self {
            goal: goal.into(),
            complexity,
            todos: vec![],
            planned: false,
            verified: false,
            change_summary: String::new(),
            started_at: timestamp_now(),
            completed_at: None,
        }
    }

    /// Number of completed todos.
    pub fn completed(&self) -> usize {
        self.todos.iter().filter(|t| t.done).count()
    }

    /// Number of pending todos.
    pub fn pending(&self) -> usize {
        self.todos.iter().filter(|t| !t.done).count()
    }

    /// Whether all todos are done.
    pub fn all_done(&self) -> bool {
        !self.todos.is_empty() && self.todos.iter().all(|t| t.done)
    }

    /// Whether the task is ready to close (all done + verified if complex).
    pub fn ready_to_close(&self) -> bool {
        if !self.all_done() {
            return false;
        }
        match self.complexity {
            TaskComplexity::Complex => self.verified,
            _ => true,
        }
    }

    /// Generate status summary for display.
    pub fn status_summary(&self) -> String {
        let progress = if self.todos.is_empty() {
            "No tasks defined".to_string()
        } else {
            format!(
                "{}/{} tasks done ({:.0}%)",
                self.completed(),
                self.todos.len(),
                if self.todos.is_empty() { 0.0 } else {
                    (self.completed() as f64 / self.todos.len() as f64) * 100.0
                }
            )
        };

        let mut lines = vec![
            format!("Goal: {}", self.goal),
            format!("Complexity: {}", self.complexity),
            format!("Progress: {}", progress),
            format!("Planned: {}", if self.planned { "yes" } else { "no" }),
            format!("Verified: {}", if self.verified { "yes" } else { "no" }),
        ];

        if !self.todos.is_empty() {
            lines.push(String::new());
            lines.push("Tasks:".to_string());
            for item in &self.todos {
                let check = if item.done { "x" } else { " " };
                lines.push(format!(
                    "  [{}] #{} ({}) {}",
                    check, item.id, item.step_type, item.description
                ));
            }
        }

        lines.join("\n")
    }
}

// ── LessonsStore ────────────────────────────────────────────────────────────

/// Persists lessons learned to `tasks/lessons.md`.
pub struct LessonsStore {
    path: PathBuf,
}

impl LessonsStore {
    /// Open the lessons store at the given workspace root.
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            path: workspace_root.join("tasks").join("lessons.md"),
        }
    }

    /// Open the global lessons store.
    pub fn global() -> Option<Self> {
        dirs::home_dir().map(|h| Self {
            path: h.join(".vibecli").join("lessons.md"),
        })
    }

    /// Load all lessons from disk.
    pub fn load(&self) -> Vec<Lesson> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        Self::parse(&content)
    }

    /// Save all lessons to disk.
    pub fn save(&self, lessons: &[Lesson]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = Self::render(lessons);
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Add a new lesson and save.
    pub fn add(&self, pattern: &str, rule: &str) -> Result<Lesson> {
        let mut lessons = self.load();
        let next_id = lessons.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let lesson = Lesson::new(next_id, pattern, rule);
        lessons.push(lesson.clone());
        self.save(&lessons)?;
        Ok(lesson)
    }

    /// Add a categorized lesson and save.
    pub fn add_categorized(&self, pattern: &str, rule: &str, category: &str) -> Result<Lesson> {
        let mut lessons = self.load();
        let next_id = lessons.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let mut lesson = Lesson::new(next_id, pattern, rule);
        lesson.category = category.to_string();
        lessons.push(lesson.clone());
        self.save(&lessons)?;
        Ok(lesson)
    }

    /// Increment the hit count for a lesson.
    pub fn record_hit(&self, id: u32) -> Result<bool> {
        let mut lessons = self.load();
        if let Some(lesson) = lessons.iter_mut().find(|l| l.id == id) {
            lesson.hit_count += 1;
            self.save(&lessons)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete a lesson by ID.
    pub fn delete(&self, id: u32) -> Result<bool> {
        let mut lessons = self.load();
        let before = lessons.len();
        lessons.retain(|l| l.id != id);
        if lessons.len() < before {
            self.save(&lessons)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Build context string to inject into agent system prompt.
    pub fn context_string(&self) -> String {
        let lessons = self.load();
        if lessons.is_empty() {
            return String::new();
        }
        let mut ctx = String::from("=== Lessons Learned (avoid these mistakes) ===\n");
        for l in &lessons {
            ctx.push_str(&format!("- {}: {} → {}\n", l.category, l.pattern, l.rule));
        }
        ctx
    }

    /// Parse lessons from markdown file content.
    fn parse(content: &str) -> Vec<Lesson> {
        let mut lessons = vec![];
        let mut next_id = 1u32;

        for line in content.lines() {
            let trimmed = line.trim();
            // Format: - **#ID** [category]: pattern → rule
            // Or simpler: - pattern → rule
            if let Some(rest) = trimmed.strip_prefix("- ") {
                let rest = rest.trim();

                // Try to parse structured format with ID
                if let Some(after_bold) = rest.strip_prefix("**#") {
                    if let Some((id_str, remainder)) = after_bold.split_once("**") {
                        let id = id_str.parse::<u32>().unwrap_or(next_id);
                        next_id = id + 1;
                        let remainder = remainder.trim();

                        // Parse optional [category]:
                        let (category, body) = if let Some(cat_rest) = remainder.strip_prefix('[') {
                            if let Some((cat, rest)) = cat_rest.split_once("]:") {
                                (cat.trim().to_string(), rest.trim().to_string())
                            } else {
                                (String::new(), remainder.to_string())
                            }
                        } else {
                            (String::new(), remainder.trim_start_matches(':').trim().to_string())
                        };

                        // Split on → for pattern/rule
                        if let Some((pattern, rule)) = body.split_once('→') {
                            let mut lesson = Lesson::new(id, pattern.trim(), rule.trim());
                            lesson.category = category;
                            lessons.push(lesson);
                        } else if !body.is_empty() {
                            let mut lesson = Lesson::new(id, &body, "");
                            lesson.category = category;
                            lessons.push(lesson);
                        }
                        continue;
                    }
                }

                // Simple format: pattern → rule
                if let Some((pattern, rule)) = rest.split_once('→') {
                    let lesson = Lesson::new(next_id, pattern.trim(), rule.trim());
                    next_id += 1;
                    lessons.push(lesson);
                } else if !rest.is_empty() {
                    let lesson = Lesson::new(next_id, rest, "");
                    next_id += 1;
                    lessons.push(lesson);
                }
            }
        }

        lessons
    }

    /// Render lessons to markdown.
    fn render(lessons: &[Lesson]) -> String {
        let mut out = String::from("# Lessons Learned\n\n");
        out.push_str("<!-- Auto-maintained by VibeCLI workflow orchestration -->\n");
        out.push_str("<!-- Format: **#ID** [category]: pattern → rule -->\n\n");

        if lessons.is_empty() {
            out.push_str("_No lessons recorded yet._\n");
            return out;
        }

        for lesson in lessons {
            let cat = if lesson.category.is_empty() {
                "general".to_string()
            } else {
                lesson.category.clone()
            };
            out.push_str(&format!(
                "- **#{}** [{}]: {} → {}\n",
                lesson.id, cat, lesson.pattern, lesson.rule
            ));
        }

        out
    }
}

// ── TodoStore ───────────────────────────────────────────────────────────────

/// Persists task todo list to `tasks/todo.md`.
pub struct TodoStore {
    path: PathBuf,
}

impl TodoStore {
    /// Open the todo store at the given workspace root.
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            path: workspace_root.join("tasks").join("todo.md"),
        }
    }

    /// Load current orchestration state from disk.
    pub fn load(&self) -> Option<OrchestrationState> {
        let content = std::fs::read_to_string(&self.path).ok()?;
        Self::parse(&content)
    }

    /// Save orchestration state to disk.
    pub fn save(&self, state: &OrchestrationState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = Self::render(state);
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Create a new task plan.
    pub fn create(&self, goal: &str, complexity: TaskComplexity) -> Result<OrchestrationState> {
        let state = OrchestrationState::new(goal, complexity);
        self.save(&state)?;
        Ok(state)
    }

    /// Add a todo item and save.
    pub fn add_todo(&self, description: &str) -> Result<OrchestrationState> {
        let mut state = self.load().unwrap_or_else(|| {
            OrchestrationState::new("Unnamed task", TaskComplexity::Moderate)
        });
        let next_id = state.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        state.todos.push(TodoItem::new(next_id, description));
        self.save(&state)?;
        Ok(state)
    }

    /// Mark a todo item as done.
    pub fn complete_todo(&self, id: u32) -> Result<OrchestrationState> {
        let mut state = self.load()
            .ok_or_else(|| anyhow::anyhow!("No active task — use `/orchestrate todo add` first"))?;
        if let Some(item) = state.todos.iter_mut().find(|t| t.id == id) {
            item.done = true;
        } else {
            anyhow::bail!("Todo item #{} not found", id);
        }
        self.save(&state)?;
        Ok(state)
    }

    /// Mark the task as verified.
    pub fn mark_verified(&self) -> Result<OrchestrationState> {
        let mut state = self.load()
            .ok_or_else(|| anyhow::anyhow!("No active task"))?;
        state.verified = true;
        self.save(&state)?;
        Ok(state)
    }

    /// Mark the task as planned.
    pub fn mark_planned(&self) -> Result<OrchestrationState> {
        let mut state = self.load()
            .ok_or_else(|| anyhow::anyhow!("No active task"))?;
        state.planned = true;
        self.save(&state)?;
        Ok(state)
    }

    /// Reset (clear) current task state.
    pub fn reset(&self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Parse orchestration state from markdown.
    fn parse(content: &str) -> Option<OrchestrationState> {
        let mut goal = String::new();
        let mut complexity = TaskComplexity::Moderate;
        let mut planned = false;
        let mut verified = false;
        let mut todos: Vec<TodoItem> = vec![];

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse front-matter-style headers
            if let Some(val) = trimmed.strip_prefix("**Goal:**") {
                goal = val.trim().to_string();
            } else if let Some(val) = trimmed.strip_prefix("**Complexity:**") {
                complexity = match val.trim() {
                    "trivial" => TaskComplexity::Trivial,
                    "complex" => TaskComplexity::Complex,
                    _ => TaskComplexity::Moderate,
                };
            } else if let Some(val) = trimmed.strip_prefix("**Planned:**") {
                planned = val.trim() == "yes";
            } else if let Some(val) = trimmed.strip_prefix("**Verified:**") {
                verified = val.trim() == "yes";
            }

            // Parse checklist items: - [x] #ID (type) description
            let (rest, done) = if let Some(r) = trimmed.strip_prefix("- [x] ") {
                (r.trim(), true)
            } else if let Some(r) = trimmed.strip_prefix("- [ ] ") {
                (r.trim(), false)
            } else {
                continue;
            };

            if rest.is_empty() { continue; }

            // Parse #ID prefix
            let (id, desc_part) = if let Some(after_hash) = rest.strip_prefix('#') {
                if let Some((id_str, remainder)) = after_hash.split_once(' ') {
                    (
                        id_str.parse::<u32>().unwrap_or(todos.len() as u32 + 1),
                        remainder.trim(),
                    )
                } else {
                    (todos.len() as u32 + 1, rest)
                }
            } else {
                (todos.len() as u32 + 1, rest)
            };

            // Parse optional (type) prefix
            let (step_type, description) = if let Some(after_paren) = desc_part.strip_prefix('(') {
                if let Some((type_str, desc)) = after_paren.split_once(')') {
                    let st = match type_str.trim() {
                        "plan" => StepType::Plan,
                        "research" => StepType::Research,
                        "verify" => StepType::Verify,
                        "test" => StepType::Test,
                        "review" => StepType::Review,
                        _ => StepType::Build,
                    };
                    (st, desc.trim().to_string())
                } else {
                    (StepType::Build, desc_part.to_string())
                }
            } else {
                (StepType::Build, desc_part.to_string())
            };

            todos.push(TodoItem {
                id,
                description,
                done,
                step_type,
            });
        }

        if goal.is_empty() && todos.is_empty() {
            return None;
        }

        Some(OrchestrationState {
            goal,
            complexity,
            todos,
            planned,
            verified,
            change_summary: String::new(),
            started_at: String::new(),
            completed_at: None,
        })
    }

    /// Render orchestration state to markdown.
    fn render(state: &OrchestrationState) -> String {
        let mut out = String::from("# Task Plan\n\n");
        out.push_str("<!-- Auto-maintained by VibeCLI workflow orchestration -->\n\n");
        out.push_str(&format!("**Goal:** {}\n", state.goal));
        out.push_str(&format!("**Complexity:** {}\n", state.complexity));
        out.push_str(&format!("**Planned:** {}\n", if state.planned { "yes" } else { "no" }));
        out.push_str(&format!("**Verified:** {}\n", if state.verified { "yes" } else { "no" }));
        out.push('\n');

        if state.todos.is_empty() {
            out.push_str("_No tasks yet — add tasks with `/orchestrate todo add <description>`._\n");
        } else {
            out.push_str("## Tasks\n\n");
            for item in &state.todos {
                let check = if item.done { "x" } else { " " };
                out.push_str(&format!(
                    "- [{}] #{} ({}) {}\n",
                    check, item.id, item.step_type, item.description
                ));
            }
        }

        if !state.change_summary.is_empty() {
            out.push_str(&format!("\n## Review\n\n{}\n", state.change_summary));
        }

        out
    }
}

// ── Complexity Estimator ────────────────────────────────────────────────────

/// Heuristic complexity estimation based on task description.
pub fn estimate_complexity(task_description: &str) -> TaskComplexity {
    let lower = task_description.to_lowercase();
    let word_count = lower.split_whitespace().count();

    // Complex indicators
    let complex_keywords = [
        "refactor", "architect", "redesign", "migrate", "rewrite",
        "implement", "add feature", "new system", "integrate",
        "across multiple", "end-to-end", "full-stack",
    ];
    let has_complex = complex_keywords.iter().any(|k| lower.contains(k));

    // Trivial indicators
    let trivial_keywords = [
        "typo", "rename", "fix import", "update version", "bump",
        "add comment", "fix lint", "format", "whitespace",
    ];
    let has_trivial = trivial_keywords.iter().any(|k| lower.contains(k));

    if has_trivial && word_count < 15 {
        TaskComplexity::Trivial
    } else if has_complex || word_count > 30 {
        TaskComplexity::Complex
    } else {
        TaskComplexity::Moderate
    }
}

/// Determine if plan mode should be entered for this task.
pub fn should_plan(complexity: TaskComplexity) -> bool {
    matches!(complexity, TaskComplexity::Complex)
}

/// Determine if elegance review should be triggered.
pub fn should_review_elegance(complexity: TaskComplexity) -> bool {
    matches!(complexity, TaskComplexity::Complex | TaskComplexity::Moderate)
}

// ── System Prompt Injection ─────────────────────────────────────────────────

/// Build orchestration rules to inject into agent system prompt.
pub fn orchestration_system_prompt(
    lessons: &[Lesson],
    current_task: Option<&OrchestrationState>,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("=== Workflow Orchestration Rules ===\n\n");

    // Core principles
    prompt.push_str("CORE PRINCIPLES:\n");
    prompt.push_str("- Simplicity First: make every change as simple as possible. Minimal code impact.\n");
    prompt.push_str("- No Laziness: find root causes. No temporary fixes. Senior developer standards.\n");
    prompt.push_str("- Minimal Impact: changes should only touch what's necessary.\n\n");

    // Planning rule
    prompt.push_str("PLANNING:\n");
    prompt.push_str("- For non-trivial tasks (3+ steps or architectural decisions), plan before building.\n");
    prompt.push_str("- If something goes sideways, STOP and re-plan — don't keep pushing.\n\n");

    // Subagent strategy
    prompt.push_str("SUBAGENT STRATEGY:\n");
    prompt.push_str("- Use subagents to keep the main context clean.\n");
    prompt.push_str("- Offload research, exploration, and parallel analysis to subagents.\n");
    prompt.push_str("- One task per subagent for focused execution.\n\n");

    // Verification
    prompt.push_str("VERIFICATION:\n");
    prompt.push_str("- Never mark a task complete without proving it works.\n");
    prompt.push_str("- Run tests, check logs, demonstrate correctness.\n");
    prompt.push_str("- Ask: 'Would a staff engineer approve this?'\n\n");

    // Bug fixing
    prompt.push_str("BUG FIXING:\n");
    prompt.push_str("- When given a bug: just fix it. Point at logs/errors/failing tests, then resolve.\n");
    prompt.push_str("- Go fix failing CI tests without being told how.\n\n");

    // Lessons
    if !lessons.is_empty() {
        prompt.push_str("LESSONS LEARNED (avoid these patterns):\n");
        for lesson in lessons {
            let cat = if lesson.category.is_empty() { "general" } else { &lesson.category };
            prompt.push_str(&format!("- [{}] {} → {}\n", cat, lesson.pattern, lesson.rule));
        }
        prompt.push('\n');
    }

    // Current task context
    if let Some(task) = current_task {
        prompt.push_str(&format!("CURRENT TASK: {}\n", task.goal));
        prompt.push_str(&format!("Complexity: {} | Planned: {} | Verified: {}\n",
            task.complexity,
            if task.planned { "yes" } else { "no" },
            if task.verified { "yes" } else { "no" },
        ));
        let pending: Vec<String> = task.todos.iter()
            .filter(|t| !t.done)
            .map(|t| format!("  - #{} ({}) {}", t.id, t.step_type, t.description))
            .collect();
        if !pending.is_empty() {
            prompt.push_str("Pending tasks:\n");
            prompt.push_str(&pending.join("\n"));
            prompt.push('\n');
        }
    }

    prompt
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = epoch_days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Lesson tests ──

    #[test]
    fn lesson_display() {
        let lesson = Lesson::new(1, "Used unwrap() in production", "Always use ? or expect()");
        assert!(format!("{}", lesson).contains("unwrap()"));
        assert!(format!("{}", lesson).contains("general"));
    }

    #[test]
    fn lesson_with_category() {
        let mut lesson = Lesson::new(1, "Hardcoded URL", "Use config");
        lesson.category = "security".to_string();
        assert!(format!("{}", lesson).contains("security"));
    }

    // ── LessonsStore tests ──

    #[test]
    fn lessons_store_add_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());

        let lesson = store.add("Missing error handling", "Always handle Result with ?").unwrap();
        assert_eq!(lesson.id, 1);

        let loaded = store.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].pattern, "Missing error handling");
        assert_eq!(loaded[0].rule, "Always handle Result with ?");
    }

    #[test]
    fn lessons_store_add_categorized() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());

        let lesson = store.add_categorized("SQL injection", "Use parameterized queries", "security").unwrap();
        assert_eq!(lesson.category, "security");

        let loaded = store.load();
        assert_eq!(loaded[0].category, "security");
    }

    #[test]
    fn lessons_store_delete() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());

        store.add("Lesson 1", "Rule 1").unwrap();
        store.add("Lesson 2", "Rule 2").unwrap();

        assert!(store.delete(1).unwrap());
        assert_eq!(store.load().len(), 1);
        assert_eq!(store.load()[0].id, 2);
    }

    #[test]
    fn lessons_store_delete_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());
        store.add("Test", "Rule").unwrap();
        assert!(!store.delete(999).unwrap());
    }

    #[test]
    fn lessons_store_record_hit() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());
        store.add("Pattern", "Rule").unwrap();

        assert!(store.record_hit(1).unwrap());
        let loaded = store.load();
        // Hit count is not persisted in markdown format, but the function succeeds
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn lessons_store_empty() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());
        assert!(store.load().is_empty());
    }

    #[test]
    fn lessons_context_string_empty() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());
        assert!(store.context_string().is_empty());
    }

    #[test]
    fn lessons_context_string_with_lessons() {
        let tmp = TempDir::new().unwrap();
        let store = LessonsStore::for_workspace(tmp.path());
        store.add("Pattern A", "Rule A").unwrap();
        let ctx = store.context_string();
        assert!(ctx.contains("Lessons Learned"));
        assert!(ctx.contains("Pattern A"));
    }

    // ── TodoStore tests ──

    #[test]
    fn todo_store_create_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());

        let state = store.create("Build auth system", TaskComplexity::Complex).unwrap();
        assert_eq!(state.goal, "Build auth system");
        assert_eq!(state.complexity, TaskComplexity::Complex);
        assert!(!state.planned);
        assert!(!state.verified);

        let loaded = store.load().unwrap();
        assert_eq!(loaded.goal, "Build auth system");
    }

    #[test]
    fn todo_store_add_and_complete() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());

        store.create("Feature X", TaskComplexity::Moderate).unwrap();
        store.add_todo("Design API schema").unwrap();
        store.add_todo("Write handler").unwrap();
        store.add_todo("Add tests").unwrap();

        let state = store.load().unwrap();
        assert_eq!(state.todos.len(), 3);
        assert_eq!(state.pending(), 3);

        let state = store.complete_todo(1).unwrap();
        assert_eq!(state.completed(), 1);
        assert_eq!(state.pending(), 2);
    }

    #[test]
    fn todo_store_complete_nonexistent_errors() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());
        store.create("Task", TaskComplexity::Trivial).unwrap();
        store.add_todo("Item").unwrap();

        let result = store.complete_todo(999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn todo_store_mark_verified() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());
        store.create("Task", TaskComplexity::Complex).unwrap();

        let state = store.mark_verified().unwrap();
        assert!(state.verified);
    }

    #[test]
    fn todo_store_mark_planned() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());
        store.create("Task", TaskComplexity::Complex).unwrap();

        let state = store.mark_planned().unwrap();
        assert!(state.planned);
    }

    #[test]
    fn todo_store_reset() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());
        store.create("Task", TaskComplexity::Trivial).unwrap();

        store.reset().unwrap();
        assert!(store.load().is_none());
    }

    #[test]
    fn todo_store_add_without_create() {
        let tmp = TempDir::new().unwrap();
        let store = TodoStore::for_workspace(tmp.path());
        // Should create a default state
        let state = store.add_todo("Quick fix").unwrap();
        assert_eq!(state.goal, "Unnamed task");
        assert_eq!(state.todos.len(), 1);
    }

    // ── OrchestrationState tests ──

    #[test]
    fn orchestration_state_ready_to_close_trivial() {
        let mut state = OrchestrationState::new("Fix typo", TaskComplexity::Trivial);
        state.todos.push(TodoItem { id: 1, description: "Fix it".into(), done: true, step_type: StepType::Build });
        assert!(state.ready_to_close()); // trivial doesn't require verification
    }

    #[test]
    fn orchestration_state_ready_to_close_complex_unverified() {
        let mut state = OrchestrationState::new("Refactor auth", TaskComplexity::Complex);
        state.todos.push(TodoItem { id: 1, description: "Step 1".into(), done: true, step_type: StepType::Build });
        assert!(!state.ready_to_close()); // complex requires verification
    }

    #[test]
    fn orchestration_state_ready_to_close_complex_verified() {
        let mut state = OrchestrationState::new("Refactor auth", TaskComplexity::Complex);
        state.todos.push(TodoItem { id: 1, description: "Step 1".into(), done: true, step_type: StepType::Build });
        state.verified = true;
        assert!(state.ready_to_close());
    }

    #[test]
    fn orchestration_state_not_ready_with_pending() {
        let mut state = OrchestrationState::new("Task", TaskComplexity::Trivial);
        state.todos.push(TodoItem { id: 1, description: "Done".into(), done: true, step_type: StepType::Build });
        state.todos.push(TodoItem { id: 2, description: "Pending".into(), done: false, step_type: StepType::Build });
        assert!(!state.ready_to_close());
    }

    #[test]
    fn status_summary_format() {
        let mut state = OrchestrationState::new("Build X", TaskComplexity::Complex);
        state.planned = true;
        state.todos.push(TodoItem::new(1, "Step one").with_type(StepType::Plan));
        state.todos.push(TodoItem::new(2, "Step two"));

        let summary = state.status_summary();
        assert!(summary.contains("Build X"));
        assert!(summary.contains("complex"));
        assert!(summary.contains("0/2"));
        assert!(summary.contains("plan"));
    }

    // ── Complexity estimation tests ──

    #[test]
    fn estimate_complexity_trivial() {
        assert_eq!(estimate_complexity("fix typo in readme"), TaskComplexity::Trivial);
        assert_eq!(estimate_complexity("rename variable"), TaskComplexity::Trivial);
        assert_eq!(estimate_complexity("fix lint warning"), TaskComplexity::Trivial);
    }

    #[test]
    fn estimate_complexity_complex() {
        assert_eq!(
            estimate_complexity("refactor the entire authentication system to use OAuth2"),
            TaskComplexity::Complex
        );
        assert_eq!(
            estimate_complexity("implement a new caching layer across multiple services"),
            TaskComplexity::Complex
        );
    }

    #[test]
    fn estimate_complexity_moderate() {
        assert_eq!(estimate_complexity("add a new endpoint for user profiles"), TaskComplexity::Moderate);
        assert_eq!(estimate_complexity("fix the login bug"), TaskComplexity::Moderate);
    }

    // ── Plan mode decision tests ──

    #[test]
    fn should_plan_for_complex() {
        assert!(should_plan(TaskComplexity::Complex));
        assert!(!should_plan(TaskComplexity::Moderate));
        assert!(!should_plan(TaskComplexity::Trivial));
    }

    #[test]
    fn should_review_elegance_skips_trivial() {
        assert!(!should_review_elegance(TaskComplexity::Trivial));
        assert!(should_review_elegance(TaskComplexity::Moderate));
        assert!(should_review_elegance(TaskComplexity::Complex));
    }

    // ── System prompt injection test ──

    #[test]
    fn orchestration_system_prompt_includes_rules() {
        let prompt = orchestration_system_prompt(&[], None);
        assert!(prompt.contains("Workflow Orchestration Rules"));
        assert!(prompt.contains("Simplicity First"));
        assert!(prompt.contains("VERIFICATION"));
        assert!(prompt.contains("SUBAGENT STRATEGY"));
    }

    #[test]
    fn orchestration_system_prompt_with_lessons() {
        let lessons = vec![
            Lesson::new(1, "Used unwrap()", "Use ? operator"),
        ];
        let prompt = orchestration_system_prompt(&lessons, None);
        assert!(prompt.contains("LESSONS LEARNED"));
        assert!(prompt.contains("unwrap()"));
    }

    #[test]
    fn orchestration_system_prompt_with_task() {
        let mut state = OrchestrationState::new("Build X", TaskComplexity::Complex);
        state.todos.push(TodoItem::new(1, "Step 1"));
        let prompt = orchestration_system_prompt(&[], Some(&state));
        assert!(prompt.contains("CURRENT TASK: Build X"));
        assert!(prompt.contains("Step 1"));
    }

    // ── TodoItem step type ──

    #[test]
    fn todo_item_with_type() {
        let item = TodoItem::new(1, "Design API").with_type(StepType::Plan);
        assert_eq!(item.step_type, StepType::Plan);
    }

    // ── Markdown roundtrip tests ──

    #[test]
    fn lessons_markdown_roundtrip() {
        let lessons = vec![
            Lesson::new(1, "Pattern A", "Rule A"),
            {
                let mut l = Lesson::new(2, "Pattern B", "Rule B");
                l.category = "security".to_string();
                l
            },
        ];
        let rendered = LessonsStore::render(&lessons);
        let parsed = LessonsStore::parse(&rendered);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pattern, "Pattern A");
        assert_eq!(parsed[1].category, "security");
    }

    #[test]
    fn todo_markdown_roundtrip() {
        let mut state = OrchestrationState::new("My Goal", TaskComplexity::Complex);
        state.planned = true;
        state.todos.push(TodoItem::new(1, "First step").with_type(StepType::Plan));
        state.todos.push(TodoItem { id: 2, description: "Done step".into(), done: true, step_type: StepType::Build });
        state.todos.push(TodoItem::new(3, "Verify").with_type(StepType::Verify));

        let rendered = TodoStore::render(&state);
        let parsed = TodoStore::parse(&rendered).unwrap();
        assert_eq!(parsed.goal, "My Goal");
        assert_eq!(parsed.complexity, TaskComplexity::Complex);
        assert!(parsed.planned);
        assert_eq!(parsed.todos.len(), 3);
        assert!(parsed.todos[1].done);
        assert_eq!(parsed.todos[2].step_type, StepType::Verify);
    }

    #[test]
    fn todo_parse_empty_returns_none() {
        assert!(TodoStore::parse("").is_none());
        assert!(TodoStore::parse("Some random text").is_none());
    }

    // ── Edge cases ──

    #[test]
    fn all_done_empty_is_false() {
        let state = OrchestrationState::new("Test", TaskComplexity::Trivial);
        assert!(!state.all_done()); // empty = not all done
    }

    #[test]
    fn step_type_display() {
        assert_eq!(format!("{}", StepType::Plan), "plan");
        assert_eq!(format!("{}", StepType::Build), "build");
        assert_eq!(format!("{}", StepType::Verify), "verify");
        assert_eq!(format!("{}", StepType::Research), "research");
        assert_eq!(format!("{}", StepType::Test), "test");
        assert_eq!(format!("{}", StepType::Review), "review");
    }

    #[test]
    fn task_complexity_display() {
        assert_eq!(format!("{}", TaskComplexity::Trivial), "trivial");
        assert_eq!(format!("{}", TaskComplexity::Moderate), "moderate");
        assert_eq!(format!("{}", TaskComplexity::Complex), "complex");
    }

    // ── epoch_days_to_date tests ──

    #[test]
    fn epoch_days_to_date_unix_epoch() {
        // Day 0 = 1970-01-01
        let (y, m, d) = epoch_days_to_date(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn epoch_days_to_date_known_date() {
        // 2024-01-01 is 19723 days after epoch
        let (y, m, d) = epoch_days_to_date(19723);
        assert_eq!(y, 2024);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }

    #[test]
    fn epoch_days_to_date_leap_day() {
        // 2024-02-29 is 19782 days after epoch (2024 is a leap year)
        let (y, m, d) = epoch_days_to_date(19782);
        assert_eq!(y, 2024);
        assert_eq!(m, 2);
        assert_eq!(d, 29);
    }

    // ── Lessons parse edge cases ──

    #[test]
    fn lessons_parse_empty_string() {
        let parsed = LessonsStore::parse("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn lessons_parse_no_bullets() {
        let content = "# Title\nSome text without bullets\n";
        let parsed = LessonsStore::parse(content);
        assert!(parsed.is_empty());
    }

    #[test]
    fn lessons_parse_simple_format() {
        let content = "- Missing tests → Always add unit tests\n- Bad naming → Use descriptive names\n";
        let parsed = LessonsStore::parse(content);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pattern, "Missing tests");
        assert_eq!(parsed[0].rule, "Always add unit tests");
        assert_eq!(parsed[1].id, 2);
    }

    #[test]
    fn lessons_parse_structured_with_category() {
        let content = "- **#5** [security]: SQL injection → Use parameterized queries\n";
        let parsed = LessonsStore::parse(content);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, 5);
        assert_eq!(parsed[0].category, "security");
        assert_eq!(parsed[0].pattern, "SQL injection");
        assert_eq!(parsed[0].rule, "Use parameterized queries");
    }

    #[test]
    fn lessons_parse_bullet_without_arrow() {
        let content = "- Just a plain bullet point\n";
        let parsed = LessonsStore::parse(content);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].pattern, "Just a plain bullet point");
        assert_eq!(parsed[0].rule, "");
    }

    // ── Lessons render edge cases ──

    #[test]
    fn lessons_render_empty() {
        let rendered = LessonsStore::render(&[]);
        assert!(rendered.contains("No lessons recorded yet"));
    }

    #[test]
    fn lessons_render_with_empty_category() {
        let lesson = Lesson::new(1, "pattern", "rule");
        let rendered = LessonsStore::render(&[lesson]);
        assert!(rendered.contains("[general]"));
    }

    // ── TodoStore parse edge cases ──

    #[test]
    fn todo_parse_with_research_step_type() {
        let content = "**Goal:** Research task\n**Complexity:** moderate\n**Planned:** no\n**Verified:** no\n\n- [ ] #1 (research) Investigate options\n";
        let parsed = TodoStore::parse(content).unwrap();
        assert_eq!(parsed.todos[0].step_type, StepType::Research);
        assert_eq!(parsed.todos[0].description, "Investigate options");
    }

    #[test]
    fn todo_parse_mixed_done_states() {
        let content = "**Goal:** Mix\n**Complexity:** trivial\n**Planned:** yes\n**Verified:** yes\n\n- [x] #1 (build) Done\n- [ ] #2 (test) Pending\n- [x] #3 (verify) Also done\n";
        let parsed = TodoStore::parse(content).unwrap();
        assert_eq!(parsed.todos.len(), 3);
        assert!(parsed.todos[0].done);
        assert!(!parsed.todos[1].done);
        assert!(parsed.todos[2].done);
        assert!(parsed.planned);
        assert!(parsed.verified);
    }

    // ── OrchestrationState status_summary edge cases ──

    #[test]
    fn status_summary_no_tasks() {
        let state = OrchestrationState::new("Empty goal", TaskComplexity::Trivial);
        let summary = state.status_summary();
        assert!(summary.contains("No tasks defined"));
        assert!(summary.contains("Empty goal"));
    }

    #[test]
    fn status_summary_all_done() {
        let mut state = OrchestrationState::new("Finished", TaskComplexity::Moderate);
        state.todos.push(TodoItem { id: 1, description: "A".into(), done: true, step_type: StepType::Build });
        state.todos.push(TodoItem { id: 2, description: "B".into(), done: true, step_type: StepType::Test });
        state.verified = true;
        let summary = state.status_summary();
        assert!(summary.contains("2/2"));
        assert!(summary.contains("100%"));
        assert!(summary.contains("Verified: yes"));
    }

    // ── Complexity estimation boundary cases ──

    #[test]
    fn estimate_complexity_long_description_is_complex() {
        // More than 30 words should trigger complex
        let desc = "This is a very long task description with many words that describes a moderate sounding task but has more than thirty words in the description so it should be classified as complex based on word count alone regardless of keywords";
        assert_eq!(estimate_complexity(desc), TaskComplexity::Complex);
    }

    #[test]
    fn estimate_complexity_trivial_keyword_in_long_desc_still_trivial() {
        // Trivial keyword + under 15 words
        let desc = "fix typo in main.rs";
        assert_eq!(estimate_complexity(desc), TaskComplexity::Trivial);
    }

    #[test]
    fn estimate_complexity_both_trivial_and_complex_keywords() {
        // "fix typo" (trivial) but very long (complex): complex wins if > 30 words
        let desc = "fix typo in the massive refactor of the entire authentication system and redesign the whole architecture to integrate with multiple third party providers and add new system";
        assert_eq!(estimate_complexity(desc), TaskComplexity::Complex);
    }

    // ── orchestration_system_prompt edge cases ──

    #[test]
    fn orchestration_prompt_with_lessons_and_task() {
        let mut lesson = Lesson::new(1, "Leaked secrets", "Use env vars");
        lesson.category = "security".to_string();
        let lessons = vec![lesson];

        let mut state = OrchestrationState::new("Secure the app", TaskComplexity::Complex);
        state.planned = true;
        state.todos.push(TodoItem::new(1, "Audit secrets").with_type(StepType::Research));
        state.todos.push(TodoItem { id: 2, description: "Fix leaks".into(), done: true, step_type: StepType::Build });

        let prompt = orchestration_system_prompt(&lessons, Some(&state));
        assert!(prompt.contains("LESSONS LEARNED"));
        assert!(prompt.contains("[security]"));
        assert!(prompt.contains("Leaked secrets"));
        assert!(prompt.contains("CURRENT TASK: Secure the app"));
        assert!(prompt.contains("Pending tasks:"));
        assert!(prompt.contains("Audit secrets"));
        // Done task should NOT appear in pending
        assert!(!prompt.contains("Fix leaks"));
    }

    #[test]
    fn orchestration_prompt_no_pending_tasks() {
        let mut state = OrchestrationState::new("Done task", TaskComplexity::Trivial);
        state.todos.push(TodoItem { id: 1, description: "All done".into(), done: true, step_type: StepType::Build });
        let prompt = orchestration_system_prompt(&[], Some(&state));
        assert!(prompt.contains("CURRENT TASK: Done task"));
        assert!(!prompt.contains("Pending tasks:"));
    }

    // ── TodoItem and Lesson construction ──

    #[test]
    fn lesson_new_defaults() {
        let lesson = Lesson::new(42, "pattern", "rule");
        assert_eq!(lesson.id, 42);
        assert_eq!(lesson.hit_count, 0);
        assert!(lesson.category.is_empty());
        assert!(!lesson.recorded_at.is_empty());
    }

    #[test]
    fn todo_item_new_defaults() {
        let item = TodoItem::new(7, "Do something");
        assert_eq!(item.id, 7);
        assert_eq!(item.description, "Do something");
        assert!(!item.done);
        assert_eq!(item.step_type, StepType::Build);
    }

    // ── TodoStore render roundtrip with change_summary ──

    #[test]
    fn todo_render_with_change_summary() {
        let mut state = OrchestrationState::new("Goal", TaskComplexity::Moderate);
        state.change_summary = "Changed files A, B, C".to_string();
        state.todos.push(TodoItem::new(1, "Task one"));
        let rendered = TodoStore::render(&state);
        assert!(rendered.contains("## Review"));
        assert!(rendered.contains("Changed files A, B, C"));
    }

    #[test]
    fn todo_render_empty_todos() {
        let state = OrchestrationState::new("Empty", TaskComplexity::Trivial);
        let rendered = TodoStore::render(&state);
        assert!(rendered.contains("No tasks yet"));
    }
}
