//! Plan Mode — separate planning phase before code execution.
//!
//! The `PlannerAgent` generates a structured `ExecutionPlan` by prompting the
//! model to reason about what steps are needed WITHOUT executing anything.
//! The caller shows the plan to the user for review/edit, then calls
//! `AgentLoop::run()` with the approved plan injected as system context.

use crate::agent::AgentContext;
use crate::provider::{AIProvider, Message, MessageRole};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── ExecutionPlan ─────────────────────────────────────────────────────────────

/// A structured plan generated before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// High-level goal the agent will accomplish.
    pub goal: String,
    /// Ordered steps to execute.
    pub steps: Vec<PlanStep>,
    /// Files the agent expects to read or modify.
    pub estimated_files: Vec<String>,
    /// Potential risks or things to watch out for.
    pub risks: Vec<String>,
}

impl ExecutionPlan {
    /// Format the plan as human-readable text for display in TUI/REPL.
    pub fn display(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("## Goal\n{}\n\n", self.goal));
        out.push_str("## Steps\n");
        for step in &self.steps {
            let icon = match step.status {
                PlanStepStatus::Pending => "⬜",
                PlanStepStatus::InProgress => "🔄",
                PlanStepStatus::Done => "✅",
                PlanStepStatus::Failed => "❌",
                PlanStepStatus::Skipped => "⏭",
            };
            let path = step
                .estimated_path
                .as_deref()
                .map(|p| format!(" (`{}`)", p))
                .unwrap_or_default();
            out.push_str(&format!(
                "{}  {}. [{}] {}{}\n",
                icon,
                step.id,
                step.tool,
                step.description,
                path
            ));
        }
        if !self.estimated_files.is_empty() {
            out.push_str("\n## Files\n");
            for f in &self.estimated_files {
                out.push_str(&format!("  - {}\n", f));
            }
        }
        if !self.risks.is_empty() {
            out.push_str("\n## Risks\n");
            for r in &self.risks {
                out.push_str(&format!("  ⚠️  {}\n", r));
            }
        }
        out
    }
}

/// A single step within an `ExecutionPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub description: String,
    /// Which tool will be used for this step.
    pub tool: String,
    /// Expected file path, if relevant.
    pub estimated_path: Option<String>,
    pub status: PlanStepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Done,
    Failed,
    Skipped,
}

// ── PlannerAgent ──────────────────────────────────────────────────────────────

/// Generates execution plans without executing any tools.
pub struct PlannerAgent {
    provider: Arc<dyn AIProvider>,
}

impl PlannerAgent {
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self { provider }
    }

    /// Generate a structured execution plan for `task` given the current context.
    /// Does NOT execute any tools — pure reasoning step.
    pub async fn plan(&self, task: &str, context: &AgentContext) -> Result<ExecutionPlan> {
        let system = build_planner_system_prompt(context);
        let user = build_planner_user_prompt(task);

        let messages = vec![
            Message { role: MessageRole::System, content: system },
            Message { role: MessageRole::User, content: user },
        ];

        let response = self.provider.chat(&messages, None).await?;

        parse_plan_from_response(&response)
    }
}

// ── Prompts ───────────────────────────────────────────────────────────────────

fn build_planner_system_prompt(context: &AgentContext) -> String {
    let mut s = String::from(
        r#"You are a software planning agent. Your ONLY job is to create a detailed execution plan.
DO NOT execute any actions. DO NOT write any code. Generate ONLY the JSON plan.

You will receive a coding task. Analyze it carefully and output a JSON object matching this exact schema:

{
  "goal": "one-sentence description of what will be accomplished",
  "steps": [
    {
      "id": 1,
      "description": "read the main source file to understand existing structure",
      "tool": "read_file",
      "estimated_path": "src/main.rs",
      "status": "pending"
    }
  ],
  "estimated_files": ["src/main.rs", "Cargo.toml"],
  "risks": ["modifying public API may break callers"]
}

Valid tool names: read_file, write_file, apply_patch, bash, search_files, list_directory, task_complete

IMPORTANT:
- Output ONLY valid JSON — no prose, no markdown, no code blocks
- Every step must specify exactly one tool
- Keep steps granular (one file operation per step)
- List at most 15 steps
- List realistic risks, not generic ones
"#,
    );

    if !context.workspace_root.as_os_str().is_empty() {
        s.push_str(&format!(
            "\nWorkspace root: {}\n",
            context.workspace_root.display()
        ));
    }
    if let Some(branch) = &context.git_branch {
        s.push_str(&format!("Git branch: {}\n", branch));
    }
    if let Some(diff) = &context.git_diff_summary {
        s.push_str(&format!("Current diff summary:\n{}\n", diff));
    }
    s
}

fn build_planner_user_prompt(task: &str) -> String {
    format!("Task: {}\n\nGenerate the execution plan JSON:", task)
}

// ── Plan Parsing ──────────────────────────────────────────────────────────────

fn parse_plan_from_response(response: &str) -> Result<ExecutionPlan> {
    // Strip markdown code fences if present
    let cleaned = strip_json_fences(response);

    // Try to parse directly
    if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(&cleaned) {
        return Ok(plan);
    }

    // Try to find JSON object in the response
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            let slice = &cleaned[start..=end];
            if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(slice) {
                return Ok(plan);
            }
        }
    }

    let end = response.char_indices().nth(500).map(|(i,_)| i).unwrap_or(response.len());
    bail!(
        "Could not parse execution plan from model response.\nResponse was:\n{}",
        &response[..end]
    )
}

fn strip_json_fences(text: &str) -> String {
    let trimmed = text.trim();
    // Remove ```json ... ``` or ``` ... ```
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
    {
        if let Some(end) = inner.rfind("```") {
            return inner[..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan_json() -> &'static str {
        r#"{
            "goal": "Add error handling to main.rs",
            "steps": [
                {"id": 1, "description": "Read main.rs", "tool": "read_file", "estimated_path": "src/main.rs", "status": "pending"},
                {"id": 2, "description": "Apply patch", "tool": "apply_patch", "estimated_path": "src/main.rs", "status": "pending"},
                {"id": 3, "description": "Run tests", "tool": "bash", "estimated_path": null, "status": "pending"}
            ],
            "estimated_files": ["src/main.rs"],
            "risks": ["may break existing error paths"]
        }"#
    }

    #[test]
    fn parse_valid_json() {
        let plan = parse_plan_from_response(sample_plan_json()).unwrap();
        assert_eq!(plan.goal, "Add error handling to main.rs");
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].tool, "read_file");
        assert_eq!(plan.estimated_files, vec!["src/main.rs"]);
        assert_eq!(plan.risks.len(), 1);
    }

    #[test]
    fn parse_json_in_markdown_fence() {
        let wrapped = format!("```json\n{}\n```", sample_plan_json());
        let plan = parse_plan_from_response(&wrapped).unwrap();
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn parse_json_with_prose_around() {
        let wrapped = format!("Here is the plan:\n{}\nLet me know!", sample_plan_json());
        let plan = parse_plan_from_response(&wrapped).unwrap();
        assert_eq!(plan.goal, "Add error handling to main.rs");
    }

    #[test]
    fn plan_display_includes_goal() {
        let plan = parse_plan_from_response(sample_plan_json()).unwrap();
        let display = plan.display();
        assert!(display.contains("Add error handling to main.rs"));
        assert!(display.contains("read_file"));
    }

    #[test]
    fn parse_fails_on_garbage() {
        assert!(parse_plan_from_response("not json at all").is_err());
    }

    // ── strip_json_fences ────────────────────────────────────────────────

    #[test]
    fn strip_json_fences_with_json_tag() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        let result = strip_json_fences(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_without_tag() {
        let input = "```\n{\"key\": \"value\"}\n```";
        let result = strip_json_fences(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_no_fences() {
        let input = "{\"key\": \"value\"}";
        let result = strip_json_fences(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_json_fences_with_whitespace() {
        let input = "  ```json\n{\"key\": 1}\n```  ";
        let result = strip_json_fences(input);
        assert_eq!(result, "{\"key\": 1}");
    }

    // ── PlanStepStatus serde ─────────────────────────────────────────────

    #[test]
    fn plan_step_status_serde_all_variants() {
        let variants = vec![
            (PlanStepStatus::Pending, "\"pending\""),
            (PlanStepStatus::InProgress, "\"in_progress\""),
            (PlanStepStatus::Done, "\"done\""),
            (PlanStepStatus::Failed, "\"failed\""),
            (PlanStepStatus::Skipped, "\"skipped\""),
        ];
        for (status, expected) in variants {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected, "serialization of {:?}", status);
            let back: PlanStepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status, "deserialization of {}", expected);
        }
    }

    // ── PlanStep serde roundtrip ─────────────────────────────────────────

    #[test]
    fn plan_step_serde_roundtrip() {
        let step = PlanStep {
            id: 1,
            description: "Read file".to_string(),
            tool: "read_file".to_string(),
            estimated_path: Some("src/main.rs".to_string()),
            status: PlanStepStatus::Pending,
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: PlanStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 1);
        assert_eq!(back.description, "Read file");
        assert_eq!(back.tool, "read_file");
        assert_eq!(back.estimated_path.as_deref(), Some("src/main.rs"));
        assert_eq!(back.status, PlanStepStatus::Pending);
    }

    #[test]
    fn plan_step_null_estimated_path() {
        let step = PlanStep {
            id: 2,
            description: "Run tests".to_string(),
            tool: "bash".to_string(),
            estimated_path: None,
            status: PlanStepStatus::Done,
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: PlanStep = serde_json::from_str(&json).unwrap();
        assert!(back.estimated_path.is_none());
    }

    // ── ExecutionPlan display variants ────────────────────────────────────

    #[test]
    fn plan_display_shows_all_step_statuses() {
        let plan = ExecutionPlan {
            goal: "Test display".to_string(),
            steps: vec![
                PlanStep { id: 1, description: "step 1".into(), tool: "read_file".into(), estimated_path: None, status: PlanStepStatus::Done },
                PlanStep { id: 2, description: "step 2".into(), tool: "bash".into(), estimated_path: None, status: PlanStepStatus::Failed },
                PlanStep { id: 3, description: "step 3".into(), tool: "write_file".into(), estimated_path: Some("out.rs".into()), status: PlanStepStatus::Skipped },
                PlanStep { id: 4, description: "step 4".into(), tool: "bash".into(), estimated_path: None, status: PlanStepStatus::InProgress },
            ],
            estimated_files: vec![],
            risks: vec![],
        };
        let display = plan.display();
        assert!(display.contains("step 1"));
        assert!(display.contains("step 2"));
        assert!(display.contains("(`out.rs`)"));
    }

    #[test]
    fn plan_display_includes_files_and_risks() {
        let plan = ExecutionPlan {
            goal: "Goal".to_string(),
            steps: vec![],
            estimated_files: vec!["src/lib.rs".into(), "Cargo.toml".into()],
            risks: vec!["May break API".into()],
        };
        let display = plan.display();
        assert!(display.contains("## Files"));
        assert!(display.contains("src/lib.rs"));
        assert!(display.contains("Cargo.toml"));
        assert!(display.contains("## Risks"));
        assert!(display.contains("May break API"));
    }

    #[test]
    fn plan_display_no_files_no_risks_sections() {
        let plan = ExecutionPlan {
            goal: "Minimal".to_string(),
            steps: vec![],
            estimated_files: vec![],
            risks: vec![],
        };
        let display = plan.display();
        assert!(!display.contains("## Files"));
        assert!(!display.contains("## Risks"));
    }

    // ── ExecutionPlan serde roundtrip ─────────────────────────────────────

    #[test]
    fn execution_plan_serde_roundtrip() {
        let plan = ExecutionPlan {
            goal: "Fix all bugs".to_string(),
            steps: vec![
                PlanStep { id: 1, description: "read".into(), tool: "read_file".into(), estimated_path: None, status: PlanStepStatus::Pending },
            ],
            estimated_files: vec!["src/main.rs".into()],
            risks: vec!["May break things".into()],
        };
        let json = serde_json::to_string(&plan).unwrap();
        let back: ExecutionPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.goal, "Fix all bugs");
        assert_eq!(back.steps.len(), 1);
        assert_eq!(back.estimated_files.len(), 1);
        assert_eq!(back.risks.len(), 1);
    }

    // ── build_planner_user_prompt ─────────────────────────────────────────

    #[test]
    fn build_planner_user_prompt_contains_task() {
        let prompt = build_planner_user_prompt("Add logging to main.rs");
        assert!(prompt.contains("Add logging to main.rs"));
        assert!(prompt.contains("Generate the execution plan JSON"));
    }
}
