//! github_action — Generate and validate VibeCLI GitHub Action workflow YAML.
//! Also provides scaffold content for the `vibecody-action/` GitHub Action.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Event triggers for a GitHub Actions workflow.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionTrigger {
    PullRequest,
    IssueComment { pattern: String },
    Push { branch: String },
    WorkflowDispatch,
}

/// A single environment variable key/value pair.
#[derive(Debug, Clone)]
pub struct ActionEnv {
    pub key: String,
    pub value: String,
}

/// A single step within a job.
#[derive(Debug, Clone)]
pub struct ActionStep {
    pub name: String,
    pub uses: Option<String>,
    pub run: Option<String>,
    pub env: Vec<ActionEnv>,
    pub if_condition: Option<String>,
}

/// A job definition within a workflow.
#[derive(Debug, Clone)]
pub struct ActionJob {
    pub id: String,
    pub name: String,
    pub runs_on: String,
    pub steps: Vec<ActionStep>,
}

/// Top-level workflow configuration.
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    pub name: String,
    pub triggers: Vec<ActionTrigger>,
    pub jobs: Vec<ActionJob>,
}

/// Factory for generating pre-built workflows and action scaffold files.
pub struct ActionGenerator;

// ---------------------------------------------------------------------------
// ActionStep
// ---------------------------------------------------------------------------

impl ActionStep {
    /// Create a checkout step using `actions/checkout@v4`.
    pub fn checkout() -> Self {
        Self {
            name: "Checkout".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: vec![],
            if_condition: None,
        }
    }

    /// Create a step that runs `vibecli -p "<prompt>"`.
    pub fn vibecli_run(prompt: impl Into<String>) -> Self {
        let p = prompt.into();
        Self {
            name: format!("Run VibeCLI: {}", p),
            uses: None,
            run: Some(format!("vibecli -p \"{}\"", p)),
            env: vec![],
            if_condition: None,
        }
    }

    /// Returns `true` if this step has an env var with the given key.
    pub fn has_env(&self, key: &str) -> bool {
        self.env.iter().any(|e| e.key == key)
    }
}

// ---------------------------------------------------------------------------
// WorkflowConfig
// ---------------------------------------------------------------------------

impl WorkflowConfig {
    /// Serialize the workflow to a human-readable YAML-like string.
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();

        // Name
        out.push_str(&format!("name: {}\n", self.name));
        out.push('\n');

        // Triggers
        out.push_str("on:\n");
        for trigger in &self.triggers {
            match trigger {
                ActionTrigger::PullRequest => {
                    out.push_str("  pull_request:\n");
                }
                ActionTrigger::IssueComment { pattern } => {
                    out.push_str("  issue_comment:\n");
                    out.push_str(&format!("    pattern: \"{}\"\n", pattern));
                }
                ActionTrigger::Push { branch } => {
                    out.push_str("  push:\n");
                    out.push_str(&format!("    branches: [{}]\n", branch));
                }
                ActionTrigger::WorkflowDispatch => {
                    out.push_str("  workflow_dispatch:\n");
                }
            }
        }
        out.push('\n');

        // Jobs
        out.push_str("jobs:\n");
        for job in &self.jobs {
            out.push_str(&format!("  {}:\n", job.id));
            out.push_str(&format!("    name: {}\n", job.name));
            out.push_str(&format!("    runs-on: {}\n", job.runs_on));
            out.push_str("    steps:\n");
            for step in &job.steps {
                out.push_str(&format!("      - name: {}\n", step.name));
                if let Some(ref uses) = step.uses {
                    out.push_str(&format!("        uses: {}\n", uses));
                }
                if let Some(ref run) = step.run {
                    out.push_str(&format!("        run: {}\n", run));
                }
                if let Some(ref cond) = step.if_condition {
                    out.push_str(&format!("        if: {}\n", cond));
                }
                if !step.env.is_empty() {
                    out.push_str("        env:\n");
                    for e in &step.env {
                        out.push_str(&format!("          {}: {}\n", e.key, e.value));
                    }
                }
            }
        }

        out
    }

    /// Validate the workflow and return a list of warnings.
    /// An empty list means the workflow is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.triggers.is_empty() {
            warnings.push("Workflow has no triggers".to_string());
        }

        if self.jobs.is_empty() {
            warnings.push("Workflow has no jobs".to_string());
        }

        for job in &self.jobs {
            if job.steps.is_empty() {
                warnings.push(format!("Job '{}' has no steps", job.id));
            }
        }

        warnings
    }

    /// Returns the number of jobs in the workflow.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Returns the total number of steps across all jobs.
    pub fn step_count(&self) -> usize {
        self.jobs.iter().map(|j| j.steps.len()).sum()
    }

    /// Returns `true` if the workflow has the given trigger.
    pub fn has_trigger(&self, trigger: &ActionTrigger) -> bool {
        self.triggers.contains(trigger)
    }
}

// ---------------------------------------------------------------------------
// ActionGenerator
// ---------------------------------------------------------------------------

impl ActionGenerator {
    /// Generate a PR review workflow.
    pub fn pr_review_workflow() -> WorkflowConfig {
        WorkflowConfig {
            name: "VibeCLI PR Review".to_string(),
            triggers: vec![ActionTrigger::PullRequest],
            jobs: vec![ActionJob {
                id: "review".to_string(),
                name: "Review PR".to_string(),
                runs_on: "ubuntu-latest".to_string(),
                steps: vec![
                    ActionStep::checkout(),
                    ActionStep::vibecli_run("Review this PR and post findings"),
                ],
            }],
        }
    }

    /// Generate an issue-to-task workflow triggered by `@vibecli` comments.
    pub fn issue_to_task_workflow() -> WorkflowConfig {
        WorkflowConfig {
            name: "VibeCLI Issue Handler".to_string(),
            triggers: vec![ActionTrigger::IssueComment {
                pattern: "@vibecli".to_string(),
            }],
            jobs: vec![ActionJob {
                id: "handle_issue".to_string(),
                name: "Handle Issue".to_string(),
                runs_on: "ubuntu-latest".to_string(),
                steps: vec![
                    ActionStep::checkout(),
                    ActionStep::vibecli_run("Handle this issue"),
                ],
            }],
        }
    }

    /// Generate `action.yml` content for the `vibecody-action/` GitHub Action.
    pub fn generate_action_yml() -> String {
        r#"name: VibeCLI Action
description: Run VibeCLI in GitHub Actions
inputs:
  prompt:
    description: Prompt to run
    required: true
  api_key:
    description: AI provider API key
    required: true
runs:
  using: docker
  image: Dockerfile
"#
        .to_string()
    }

    /// Generate `entrypoint.sh` content for the `vibecody-action/` GitHub Action.
    pub fn generate_entrypoint_sh() -> String {
        r#"#!/bin/bash
set -e
vibecli -p "$INPUT_PROMPT"
"#
        .to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_review_workflow_has_pr_trigger() {
        let wf = ActionGenerator::pr_review_workflow();
        assert!(wf.has_trigger(&ActionTrigger::PullRequest));
    }

    #[test]
    fn test_pr_review_workflow_has_checkout_step() {
        let wf = ActionGenerator::pr_review_workflow();
        let has_checkout = wf
            .jobs
            .iter()
            .flat_map(|j| &j.steps)
            .any(|s| s.uses.as_deref() == Some("actions/checkout@v4"));
        assert!(has_checkout, "Expected checkout step with actions/checkout@v4");
    }

    #[test]
    fn test_issue_workflow_has_issue_comment_trigger() {
        let wf = ActionGenerator::issue_to_task_workflow();
        let has = wf.triggers.iter().any(|t| {
            matches!(t, ActionTrigger::IssueComment { pattern } if pattern == "@vibecli")
        });
        assert!(has, "Expected IssueComment trigger with pattern @vibecli");
    }

    #[test]
    fn test_workflow_to_yaml_contains_name() {
        let wf = ActionGenerator::pr_review_workflow();
        let yaml = wf.to_yaml();
        assert!(yaml.contains("name: VibeCLI PR Review"), "YAML: {}", yaml);
    }

    #[test]
    fn test_workflow_to_yaml_contains_jobs() {
        let wf = ActionGenerator::pr_review_workflow();
        let yaml = wf.to_yaml();
        assert!(yaml.contains("jobs:"), "YAML missing 'jobs:': {}", yaml);
    }

    #[test]
    fn test_validate_empty_jobs_returns_warning() {
        let wf = WorkflowConfig {
            name: "Empty".to_string(),
            triggers: vec![ActionTrigger::PullRequest],
            jobs: vec![],
        };
        let warnings = wf.validate();
        assert!(
            warnings.iter().any(|w| w.contains("no jobs")),
            "Expected 'no jobs' warning, got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validate_valid_workflow_no_warnings() {
        let wf = ActionGenerator::pr_review_workflow();
        let warnings = wf.validate();
        assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn test_generate_action_yml_contains_inputs() {
        let yml = ActionGenerator::generate_action_yml();
        assert!(yml.contains("inputs:"), "Missing 'inputs:'");
        assert!(yml.contains("prompt:"), "Missing 'prompt:'");
        assert!(yml.contains("api_key:"), "Missing 'api_key:'");
        assert!(yml.contains("using: docker"), "Missing 'using: docker'");
    }

    #[test]
    fn test_generate_entrypoint_contains_vibecli() {
        let sh = ActionGenerator::generate_entrypoint_sh();
        assert!(sh.contains("vibecli"), "Missing vibecli call");
        assert!(sh.contains("#!/bin/bash"), "Missing shebang");
        assert!(sh.contains("INPUT_PROMPT"), "Missing INPUT_PROMPT");
    }
}
