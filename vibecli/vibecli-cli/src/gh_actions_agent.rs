#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manages VibeCLI agent execution within GitHub Actions workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhActionsAgent {
    secrets: Vec<SecretEntry>,
    default_model: String,
    default_max_turns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretEntry {
    name: String,
    description: String,
}

/// Configuration for a complete GitHub Actions workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub triggers: Vec<String>,
    pub jobs: Vec<JobConfig>,
    pub env_vars: HashMap<String, String>,
}

/// Configuration for a single job within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub name: String,
    pub runs_on: String,
    pub steps: Vec<StepConfig>,
    pub needs: Vec<String>,
    pub timeout_minutes: u32,
}

/// Configuration for a single step within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    pub name: String,
    pub uses: Option<String>,
    pub run: Option<String>,
    pub env: HashMap<String, String>,
}

/// Trigger types for GitHub Actions workflows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionsTrigger {
    Push,
    PullRequest,
    Schedule,
    WorkflowDispatch,
    IssueComment,
    Release,
}

impl ActionsTrigger {
    fn as_yaml_key(&self) -> &str {
        match self {
            ActionsTrigger::Push => "push",
            ActionsTrigger::PullRequest => "pull_request",
            ActionsTrigger::Schedule => "schedule",
            ActionsTrigger::WorkflowDispatch => "workflow_dispatch",
            ActionsTrigger::IssueComment => "issue_comment",
            ActionsTrigger::Release => "release",
        }
    }
}

/// Represents a VibeCLI agent step to be embedded in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub task: String,
    pub model: String,
    pub max_turns: u32,
}

/// A validation issue found in workflow YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub line: usize,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Predefined workflow templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowTemplate {
    CodeReview,
    AutoFix,
    TestSuite,
    SecurityScan,
    Deploy,
    Custom,
}

impl WorkflowTemplate {
    fn template_name(&self) -> &str {
        match self {
            WorkflowTemplate::CodeReview => "Code Review",
            WorkflowTemplate::AutoFix => "Auto Fix",
            WorkflowTemplate::TestSuite => "Test Suite",
            WorkflowTemplate::SecurityScan => "Security Scan",
            WorkflowTemplate::Deploy => "Deploy",
            WorkflowTemplate::Custom => "Custom",
        }
    }

    fn default_triggers(&self) -> Vec<String> {
        match self {
            WorkflowTemplate::CodeReview => vec!["pull_request".into()],
            WorkflowTemplate::AutoFix => vec!["push".into()],
            WorkflowTemplate::TestSuite => vec!["push".into(), "pull_request".into()],
            WorkflowTemplate::SecurityScan => vec!["push".into(), "schedule".into()],
            WorkflowTemplate::Deploy => vec!["push".into()],
            WorkflowTemplate::Custom => vec!["workflow_dispatch".into()],
        }
    }
}

impl GhActionsAgent {
    /// Creates a new GhActionsAgent with default settings.
    pub fn new() -> Self {
        Self {
            secrets: Vec::new(),
            default_model: "claude-sonnet".to_string(),
            default_max_turns: 10,
        }
    }

    /// Generates a complete workflow YAML string from the given config.
    pub fn generate_workflow(&self, config: &WorkflowConfig) -> String {
        let mut yaml = String::with_capacity(1024);
        yaml.push_str(&format!("name: {}\n", config.name));
        yaml.push_str("\non:\n");
        for trigger in &config.triggers {
            yaml.push_str(&format!("  {}:\n", trigger));
        }

        if !config.env_vars.is_empty() {
            yaml.push_str("\nenv:\n");
            let mut keys: Vec<&String> = config.env_vars.keys().collect();
            keys.sort();
            for key in keys {
                yaml.push_str(&format!("  {}: {}\n", key, config.env_vars[key]));
            }
        }

        yaml.push_str("\njobs:\n");
        for job in &config.jobs {
            let job_id = job.name.to_lowercase().replace(' ', "-");
            yaml.push_str(&format!("  {}:\n", job_id));
            yaml.push_str(&format!("    name: {}\n", job.name));
            yaml.push_str(&format!("    runs-on: {}\n", job.runs_on));
            if job.timeout_minutes > 0 {
                yaml.push_str(&format!("    timeout-minutes: {}\n", job.timeout_minutes));
            }
            if !job.needs.is_empty() {
                yaml.push_str("    needs:\n");
                for need in &job.needs {
                    yaml.push_str(&format!("      - {}\n", need));
                }
            }
            yaml.push_str("    steps:\n");
            for step in &job.steps {
                yaml.push_str(&format!("      - name: {}\n", step.name));
                if let Some(uses) = &step.uses {
                    yaml.push_str(&format!("        uses: {}\n", uses));
                }
                if let Some(run) = &step.run {
                    if run.contains('\n') {
                        yaml.push_str("        run: |\n");
                        for line in run.lines() {
                            yaml.push_str(&format!("          {}\n", line));
                        }
                    } else {
                        yaml.push_str(&format!("        run: {}\n", run));
                    }
                }
                if !step.env.is_empty() {
                    yaml.push_str("        env:\n");
                    let mut env_keys: Vec<&String> = step.env.keys().collect();
                    env_keys.sort();
                    for key in env_keys {
                        yaml.push_str(&format!("          {}: {}\n", key, step.env[key]));
                    }
                }
            }
        }

        yaml
    }

    /// Generates a StepConfig that runs the VibeCLI agent for the given task.
    pub fn generate_agent_step(&self, task: &str, model: &str, max_turns: u32) -> StepConfig {
        let run_cmd = format!(
            "vibecli agent --task \"{}\" --model {} --max-turns {} --non-interactive",
            task, model, max_turns
        );
        let mut env = HashMap::new();
        env.insert(
            "ANTHROPIC_API_KEY".to_string(),
            "${{ secrets.ANTHROPIC_API_KEY }}".to_string(),
        );

        StepConfig {
            name: format!("Run VibeCLI Agent: {}", truncate_task(task, 40)),
            uses: None,
            run: Some(run_cmd),
            env,
        }
    }

    /// Generates a code review workflow for the given branch pattern.
    pub fn generate_review_workflow(&self, branch_pattern: &str) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let install_step = StepConfig {
            name: "Install VibeCLI".to_string(),
            uses: None,
            run: Some("curl -fsSL https://vibecody.dev/install.sh | sh".to_string()),
            env: HashMap::new(),
        };

        let review_step = self.generate_agent_step(
            "Review the pull request changes, check for bugs, suggest improvements",
            &self.default_model,
            self.default_max_turns,
        );

        let job = JobConfig {
            name: "Code Review".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step, install_step, review_step],
            needs: vec![],
            timeout_minutes: 15,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Code Review".to_string(),
            triggers: vec![format!("pull_request:\n    branches:\n      - {}", branch_pattern)],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        // Build custom YAML for the review workflow with proper trigger nesting
        let mut yaml = String::with_capacity(512);
        yaml.push_str("name: VibeCLI Code Review\n\non:\n  pull_request:\n");
        yaml.push_str(&format!("    branches:\n      - {}\n", branch_pattern));
        yaml.push_str("\njobs:\n");

        // Reuse the job rendering from generate_workflow by building a simplified config
        let simple_config = WorkflowConfig {
            name: config.name.clone(),
            triggers: vec![],
            jobs: config.jobs.clone(),
            env_vars: HashMap::new(),
        };
        let full = self.generate_workflow(&simple_config);
        // Extract everything after "jobs:\n"
        if let Some(idx) = full.find("jobs:\n") {
            yaml.push_str(&full[idx + 6..]);
        }

        yaml
    }

    /// Generates an autofix workflow that runs on push and attempts to fix issues.
    pub fn generate_autofix_workflow(&self) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let install_step = StepConfig {
            name: "Install VibeCLI".to_string(),
            uses: None,
            run: Some("curl -fsSL https://vibecody.dev/install.sh | sh".to_string()),
            env: HashMap::new(),
        };

        let fix_step = self.generate_agent_step(
            "Analyze failing tests and lint errors, apply fixes, commit changes",
            &self.default_model,
            15,
        );

        let push_step = StepConfig {
            name: "Push fixes".to_string(),
            uses: None,
            run: Some(
                "git config user.name 'vibecli-bot'\ngit config user.email 'bot@vibecody.dev'\ngit add -A\ngit diff --cached --quiet || git commit -m 'fix: auto-fix by VibeCLI agent'\ngit push"
                    .to_string(),
            ),
            env: HashMap::new(),
        };

        let job = JobConfig {
            name: "Auto Fix".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step, install_step, fix_step, push_step],
            needs: vec![],
            timeout_minutes: 20,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Auto Fix".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        self.generate_workflow(&config)
    }

    /// Generates a test workflow that runs the project test suite.
    pub fn generate_test_workflow(&self) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let install_step = StepConfig {
            name: "Install VibeCLI".to_string(),
            uses: None,
            run: Some("curl -fsSL https://vibecody.dev/install.sh | sh".to_string()),
            env: HashMap::new(),
        };

        let test_step = self.generate_agent_step(
            "Run the test suite, report failures, suggest fixes for broken tests",
            &self.default_model,
            self.default_max_turns,
        );

        let job = JobConfig {
            name: "Test Suite".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step, install_step, test_step],
            needs: vec![],
            timeout_minutes: 30,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Test Suite".to_string(),
            triggers: vec!["push".to_string(), "pull_request".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        self.generate_workflow(&config)
    }

    /// Registers a required secret with a description.
    pub fn add_secret(&mut self, name: &str, description: &str) {
        // Avoid duplicates
        if !self.secrets.iter().any(|s| s.name == name) {
            self.secrets.push(SecretEntry {
                name: name.to_string(),
                description: description.to_string(),
            });
        }
    }

    /// Returns the list of required secret names.
    pub fn list_required_secrets(&self) -> Vec<String> {
        self.secrets.iter().map(|s| s.name.clone()).collect()
    }

    /// Validates workflow YAML and returns any issues found.
    pub fn validate_workflow(&self, yaml: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for required top-level keys
        if !yaml.contains("name:") {
            issues.push(ValidationIssue {
                line: 1,
                message: "Workflow missing 'name' field".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if !yaml.contains("on:") {
            issues.push(ValidationIssue {
                line: 1,
                message: "Workflow missing 'on' trigger section".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if !yaml.contains("jobs:") {
            issues.push(ValidationIssue {
                line: 1,
                message: "Workflow missing 'jobs' section".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        // Check for common issues line by line
        for (i, line) in yaml.lines().enumerate() {
            let line_num = i + 1;

            // Check for tab characters (YAML should use spaces)
            if line.contains('\t') {
                issues.push(ValidationIssue {
                    line: line_num,
                    message: "Tab character found; YAML requires spaces for indentation".to_string(),
                    severity: ValidationSeverity::Error,
                });
            }

            // Warn about hardcoded secrets
            if line.contains("api_key") || line.contains("API_KEY") {
                if !line.contains("secrets.") && !line.contains("${{") {
                    issues.push(ValidationIssue {
                        line: line_num,
                        message: "Possible hardcoded secret; use ${{ secrets.* }} instead"
                            .to_string(),
                        severity: ValidationSeverity::Warning,
                    });
                }
            }

            // Check for missing timeout
            if line.trim_start().starts_with("runs-on:") {
                // Look ahead for timeout-minutes in the next few lines
                let remaining: String = yaml.lines().skip(i).take(5).collect::<Vec<_>>().join(" ");
                if !remaining.contains("timeout-minutes") {
                    issues.push(ValidationIssue {
                        line: line_num,
                        message: "Job has no timeout-minutes; consider adding one".to_string(),
                        severity: ValidationSeverity::Info,
                    });
                }
            }
        }

        issues
    }

    /// Estimates the number of GitHub Actions minutes a workflow will consume.
    pub fn estimate_minutes(&self, config: &WorkflowConfig) -> u32 {
        let mut total: u32 = 0;

        for job in &config.jobs {
            if job.timeout_minutes > 0 {
                // Estimate as ~60% of timeout for typical runs
                let estimated = (job.timeout_minutes as f64 * 0.6).ceil() as u32;
                total = total.saturating_add(estimated);
            } else {
                // Default estimate per step
                let step_minutes = job.steps.len() as u32 * 2;
                total = total.saturating_add(step_minutes.max(1));
            }

            // Apply runner multiplier (Linux=1x, macOS=10x, Windows=2x)
            let multiplier = runner_cost_multiplier(&job.runs_on);
            total = ((total as f64) * multiplier).ceil() as u32;
        }

        total.max(1)
    }

    /// Generates a workflow from a predefined template.
    pub fn generate_from_template(&self, template: WorkflowTemplate) -> String {
        match template {
            WorkflowTemplate::CodeReview => self.generate_review_workflow("main"),
            WorkflowTemplate::AutoFix => self.generate_autofix_workflow(),
            WorkflowTemplate::TestSuite => self.generate_test_workflow(),
            WorkflowTemplate::SecurityScan => self.generate_security_scan_workflow(),
            WorkflowTemplate::Deploy => self.generate_deploy_workflow(),
            WorkflowTemplate::Custom => self.generate_custom_workflow(),
        }
    }

    fn generate_security_scan_workflow(&self) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let install_step = StepConfig {
            name: "Install VibeCLI".to_string(),
            uses: None,
            run: Some("curl -fsSL https://vibecody.dev/install.sh | sh".to_string()),
            env: HashMap::new(),
        };

        let scan_step = self.generate_agent_step(
            "Scan the codebase for security vulnerabilities, check dependencies for CVEs",
            &self.default_model,
            self.default_max_turns,
        );

        let job = JobConfig {
            name: "Security Scan".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step, install_step, scan_step],
            needs: vec![],
            timeout_minutes: 20,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Security Scan".to_string(),
            triggers: vec!["push".to_string(), "schedule".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        self.generate_workflow(&config)
    }

    fn generate_deploy_workflow(&self) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let install_step = StepConfig {
            name: "Install VibeCLI".to_string(),
            uses: None,
            run: Some("curl -fsSL https://vibecody.dev/install.sh | sh".to_string()),
            env: HashMap::new(),
        };

        let deploy_step = self.generate_agent_step(
            "Build the project, run tests, deploy to production if all checks pass",
            &self.default_model,
            20,
        );

        let job = JobConfig {
            name: "Deploy".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step, install_step, deploy_step],
            needs: vec![],
            timeout_minutes: 30,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Deploy".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        self.generate_workflow(&config)
    }

    fn generate_custom_workflow(&self) -> String {
        let checkout_step = StepConfig {
            name: "Checkout code".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };

        let job = JobConfig {
            name: "Custom".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![checkout_step],
            needs: vec![],
            timeout_minutes: 10,
        };

        let config = WorkflowConfig {
            name: "VibeCLI Custom Workflow".to_string(),
            triggers: vec!["workflow_dispatch".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };

        self.generate_workflow(&config)
    }
}

fn truncate_task(task: &str, max_len: usize) -> &str {
    if task.len() <= max_len {
        task
    } else {
        &task[..max_len]
    }
}

fn runner_cost_multiplier(runs_on: &str) -> f64 {
    if runs_on.contains("macos") {
        10.0
    } else if runs_on.contains("windows") {
        2.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent() {
        let agent = GhActionsAgent::new();
        assert_eq!(agent.default_model, "claude-sonnet");
        assert_eq!(agent.default_max_turns, 10);
        assert!(agent.secrets.is_empty());
    }

    #[test]
    fn test_generate_workflow_basic() {
        let agent = GhActionsAgent::new();
        let config = WorkflowConfig {
            name: "Test CI".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("name: Test CI"));
        assert!(yaml.contains("push:"));
        assert!(yaml.contains("jobs:"));
    }

    #[test]
    fn test_generate_workflow_with_env_vars() {
        let agent = GhActionsAgent::new();
        let mut env = HashMap::new();
        env.insert("NODE_VERSION".to_string(), "18".to_string());
        let config = WorkflowConfig {
            name: "Build".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![],
            env_vars: env,
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("env:"));
        assert!(yaml.contains("NODE_VERSION: 18"));
    }

    #[test]
    fn test_generate_workflow_with_job() {
        let agent = GhActionsAgent::new();
        let step = StepConfig {
            name: "Run tests".to_string(),
            uses: None,
            run: Some("cargo test".to_string()),
            env: HashMap::new(),
        };
        let job = JobConfig {
            name: "Build".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![step],
            needs: vec![],
            timeout_minutes: 10,
        };
        let config = WorkflowConfig {
            name: "CI".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("name: Build"));
        assert!(yaml.contains("runs-on: ubuntu-latest"));
        assert!(yaml.contains("timeout-minutes: 10"));
        assert!(yaml.contains("run: cargo test"));
    }

    #[test]
    fn test_generate_workflow_with_needs() {
        let agent = GhActionsAgent::new();
        let job = JobConfig {
            name: "Deploy".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![],
            needs: vec!["build".to_string(), "test".to_string()],
            timeout_minutes: 5,
        };
        let config = WorkflowConfig {
            name: "Pipeline".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("needs:"));
        assert!(yaml.contains("- build"));
        assert!(yaml.contains("- test"));
    }

    #[test]
    fn test_generate_agent_step() {
        let agent = GhActionsAgent::new();
        let step = agent.generate_agent_step("Fix bugs", "claude-sonnet", 5);
        assert!(step.name.contains("VibeCLI Agent"));
        assert!(step.run.as_ref().unwrap().contains("--task"));
        assert!(step.run.as_ref().unwrap().contains("Fix bugs"));
        assert!(step.run.as_ref().unwrap().contains("--model claude-sonnet"));
        assert!(step.run.as_ref().unwrap().contains("--max-turns 5"));
        assert!(step.run.as_ref().unwrap().contains("--non-interactive"));
        assert!(step.env.contains_key("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_generate_review_workflow() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_review_workflow("main");
        assert!(yaml.contains("Code Review"));
        assert!(yaml.contains("pull_request"));
        assert!(yaml.contains("main"));
        assert!(yaml.contains("actions/checkout@v4"));
        assert!(yaml.contains("vibecli agent"));
    }

    #[test]
    fn test_generate_autofix_workflow() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_autofix_workflow();
        assert!(yaml.contains("Auto Fix"));
        assert!(yaml.contains("push:"));
        assert!(yaml.contains("git commit"));
        assert!(yaml.contains("vibecli agent"));
    }

    #[test]
    fn test_generate_test_workflow() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_test_workflow();
        assert!(yaml.contains("Test Suite"));
        assert!(yaml.contains("push:"));
        assert!(yaml.contains("pull_request:"));
        assert!(yaml.contains("timeout-minutes: 30"));
    }

    #[test]
    fn test_add_secret() {
        let mut agent = GhActionsAgent::new();
        agent.add_secret("ANTHROPIC_API_KEY", "Anthropic API key for Claude");
        agent.add_secret("DEPLOY_TOKEN", "Token for deployment");
        assert_eq!(agent.secrets.len(), 2);
    }

    #[test]
    fn test_add_secret_no_duplicates() {
        let mut agent = GhActionsAgent::new();
        agent.add_secret("ANTHROPIC_API_KEY", "Key 1");
        agent.add_secret("ANTHROPIC_API_KEY", "Key 2");
        assert_eq!(agent.secrets.len(), 1);
    }

    #[test]
    fn test_list_required_secrets() {
        let mut agent = GhActionsAgent::new();
        agent.add_secret("SECRET_A", "First");
        agent.add_secret("SECRET_B", "Second");
        let secrets = agent.list_required_secrets();
        assert_eq!(secrets, vec!["SECRET_A", "SECRET_B"]);
    }

    #[test]
    fn test_list_required_secrets_empty() {
        let agent = GhActionsAgent::new();
        assert!(agent.list_required_secrets().is_empty());
    }

    #[test]
    fn test_validate_workflow_valid() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_test_workflow();
        let issues: Vec<_> = agent
            .validate_workflow(&yaml)
            .into_iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(issues.is_empty(), "Valid workflow should have no errors");
    }

    #[test]
    fn test_validate_workflow_missing_name() {
        let agent = GhActionsAgent::new();
        let yaml = "on:\n  push:\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let issues = agent.validate_workflow(yaml);
        assert!(issues.iter().any(|i| i.message.contains("name")));
    }

    #[test]
    fn test_validate_workflow_missing_on() {
        let agent = GhActionsAgent::new();
        let yaml = "name: Test\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let issues = agent.validate_workflow(yaml);
        assert!(issues.iter().any(|i| i.message.contains("on")));
    }

    #[test]
    fn test_validate_workflow_missing_jobs() {
        let agent = GhActionsAgent::new();
        let yaml = "name: Test\non:\n  push:\n";
        let issues = agent.validate_workflow(yaml);
        assert!(issues.iter().any(|i| i.message.contains("jobs")));
    }

    #[test]
    fn test_validate_workflow_tabs() {
        let agent = GhActionsAgent::new();
        let yaml = "name: Test\non:\n\tpush:\njobs:\n\tbuild:\n";
        let issues = agent.validate_workflow(yaml);
        assert!(issues.iter().any(|i| i.message.contains("Tab")));
    }

    #[test]
    fn test_validate_workflow_hardcoded_secret() {
        let agent = GhActionsAgent::new();
        let yaml = "name: T\non:\n  push:\njobs:\n  b:\n    env:\n      API_KEY: sk-1234\n";
        let issues = agent.validate_workflow(yaml);
        assert!(issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Warning
                && i.message.contains("secret")));
    }

    #[test]
    fn test_estimate_minutes_basic() {
        let agent = GhActionsAgent::new();
        let config = WorkflowConfig {
            name: "CI".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![JobConfig {
                name: "Build".to_string(),
                runs_on: "ubuntu-latest".to_string(),
                steps: vec![],
                needs: vec![],
                timeout_minutes: 10,
            }],
            env_vars: HashMap::new(),
        };
        let est = agent.estimate_minutes(&config);
        assert!(est >= 1);
        assert!(est <= 10);
    }

    #[test]
    fn test_estimate_minutes_macos_multiplier() {
        let agent = GhActionsAgent::new();
        let linux_config = WorkflowConfig {
            name: "CI".to_string(),
            triggers: vec![],
            jobs: vec![JobConfig {
                name: "Build".to_string(),
                runs_on: "ubuntu-latest".to_string(),
                steps: vec![],
                needs: vec![],
                timeout_minutes: 10,
            }],
            env_vars: HashMap::new(),
        };
        let mac_config = WorkflowConfig {
            name: "CI".to_string(),
            triggers: vec![],
            jobs: vec![JobConfig {
                name: "Build".to_string(),
                runs_on: "macos-latest".to_string(),
                steps: vec![],
                needs: vec![],
                timeout_minutes: 10,
            }],
            env_vars: HashMap::new(),
        };
        let linux_est = agent.estimate_minutes(&linux_config);
        let mac_est = agent.estimate_minutes(&mac_config);
        assert!(mac_est > linux_est);
    }

    #[test]
    fn test_actions_trigger_yaml_key() {
        assert_eq!(ActionsTrigger::Push.as_yaml_key(), "push");
        assert_eq!(ActionsTrigger::PullRequest.as_yaml_key(), "pull_request");
        assert_eq!(ActionsTrigger::Schedule.as_yaml_key(), "schedule");
        assert_eq!(
            ActionsTrigger::WorkflowDispatch.as_yaml_key(),
            "workflow_dispatch"
        );
        assert_eq!(ActionsTrigger::IssueComment.as_yaml_key(), "issue_comment");
        assert_eq!(ActionsTrigger::Release.as_yaml_key(), "release");
    }

    #[test]
    fn test_workflow_template_names() {
        assert_eq!(WorkflowTemplate::CodeReview.template_name(), "Code Review");
        assert_eq!(WorkflowTemplate::AutoFix.template_name(), "Auto Fix");
        assert_eq!(WorkflowTemplate::Deploy.template_name(), "Deploy");
    }

    #[test]
    fn test_workflow_template_default_triggers() {
        let triggers = WorkflowTemplate::CodeReview.default_triggers();
        assert!(triggers.contains(&"pull_request".to_string()));

        let triggers = WorkflowTemplate::TestSuite.default_triggers();
        assert!(triggers.contains(&"push".to_string()));
        assert!(triggers.contains(&"pull_request".to_string()));
    }

    #[test]
    fn test_generate_from_template_code_review() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_from_template(WorkflowTemplate::CodeReview);
        assert!(yaml.contains("Code Review"));
        assert!(yaml.contains("pull_request"));
    }

    #[test]
    fn test_generate_from_template_security_scan() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_from_template(WorkflowTemplate::SecurityScan);
        assert!(yaml.contains("Security Scan"));
        assert!(yaml.contains("vulnerabilities"));
    }

    #[test]
    fn test_generate_from_template_deploy() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_from_template(WorkflowTemplate::Deploy);
        assert!(yaml.contains("Deploy"));
        assert!(yaml.contains("production"));
    }

    #[test]
    fn test_generate_from_template_custom() {
        let agent = GhActionsAgent::new();
        let yaml = agent.generate_from_template(WorkflowTemplate::Custom);
        assert!(yaml.contains("Custom"));
        assert!(yaml.contains("workflow_dispatch"));
    }

    #[test]
    fn test_multiline_run_command() {
        let agent = GhActionsAgent::new();
        let step = StepConfig {
            name: "Multi-line".to_string(),
            uses: None,
            run: Some("echo hello\necho world".to_string()),
            env: HashMap::new(),
        };
        let job = JobConfig {
            name: "Test".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![step],
            needs: vec![],
            timeout_minutes: 5,
        };
        let config = WorkflowConfig {
            name: "Multi".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("run: |"));
    }

    #[test]
    fn test_step_with_uses_action() {
        let agent = GhActionsAgent::new();
        let step = StepConfig {
            name: "Checkout".to_string(),
            uses: Some("actions/checkout@v4".to_string()),
            run: None,
            env: HashMap::new(),
        };
        let job = JobConfig {
            name: "Build".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![step],
            needs: vec![],
            timeout_minutes: 5,
        };
        let config = WorkflowConfig {
            name: "CI".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("uses: actions/checkout@v4"));
    }

    #[test]
    fn test_truncate_task() {
        assert_eq!(truncate_task("short", 10), "short");
        assert_eq!(truncate_task("a long task name here", 10), "a long tas");
    }

    #[test]
    fn test_runner_cost_multiplier() {
        assert_eq!(runner_cost_multiplier("ubuntu-latest"), 1.0);
        assert_eq!(runner_cost_multiplier("macos-latest"), 10.0);
        assert_eq!(runner_cost_multiplier("windows-latest"), 2.0);
    }

    #[test]
    fn test_validation_severity_variants() {
        let e = ValidationSeverity::Error;
        let w = ValidationSeverity::Warning;
        let i = ValidationSeverity::Info;
        assert_ne!(e, w);
        assert_ne!(w, i);
        assert_ne!(e, i);
    }

    #[test]
    fn test_agent_step_struct() {
        let step = AgentStep {
            task: "review code".to_string(),
            model: "claude-sonnet".to_string(),
            max_turns: 5,
        };
        assert_eq!(step.task, "review code");
        assert_eq!(step.max_turns, 5);
    }

    #[test]
    fn test_step_env_in_yaml() {
        let agent = GhActionsAgent::new();
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "value".to_string());
        let step = StepConfig {
            name: "With env".to_string(),
            uses: None,
            run: Some("echo $MY_VAR".to_string()),
            env,
        };
        let job = JobConfig {
            name: "Test".to_string(),
            runs_on: "ubuntu-latest".to_string(),
            steps: vec![step],
            needs: vec![],
            timeout_minutes: 5,
        };
        let config = WorkflowConfig {
            name: "Env Test".to_string(),
            triggers: vec!["push".to_string()],
            jobs: vec![job],
            env_vars: HashMap::new(),
        };
        let yaml = agent.generate_workflow(&config);
        assert!(yaml.contains("MY_VAR: value"));
    }
}
