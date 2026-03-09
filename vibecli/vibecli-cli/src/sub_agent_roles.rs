//! Specialized sub-agent roles — typed agent roles with domain-specific prompts.
//!
//! Closes P2 Gap 14: Spawn specialized sub-agents (e.g., security reviewer, test writer).

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Agent role types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum AgentRole {
    CodeReviewer,
    TestWriter,
    SecurityReviewer,
    Refactorer,
    DocumentationWriter,
    Debugger,
    Architect,
    PerformanceOptimizer,
    DependencyManager,
    MigrationSpecialist,
    Custom(String),
}

impl AgentRole {
    pub fn as_str(&self) -> &str {
        match self {
            AgentRole::CodeReviewer => "code_reviewer",
            AgentRole::TestWriter => "test_writer",
            AgentRole::SecurityReviewer => "security_reviewer",
            AgentRole::Refactorer => "refactorer",
            AgentRole::DocumentationWriter => "documentation_writer",
            AgentRole::Debugger => "debugger",
            AgentRole::Architect => "architect",
            AgentRole::PerformanceOptimizer => "performance_optimizer",
            AgentRole::DependencyManager => "dependency_manager",
            AgentRole::MigrationSpecialist => "migration_specialist",
            AgentRole::Custom(s) => s,
        }
    }

    pub fn system_prompt(&self) -> &str {
        match self {
            AgentRole::CodeReviewer => "You are a code reviewer. Analyze code for correctness, readability, and best practices. Provide actionable feedback with specific line references.",
            AgentRole::TestWriter => "You are a test writer. Generate comprehensive unit tests, integration tests, and edge case tests. Ensure high coverage and meaningful assertions.",
            AgentRole::SecurityReviewer => "You are a security reviewer. Identify vulnerabilities including OWASP Top 10, hardcoded secrets, injection flaws, and authentication issues. Suggest fixes.",
            AgentRole::Refactorer => "You are a code refactorer. Improve code structure, reduce duplication, extract functions, and apply design patterns while preserving behavior.",
            AgentRole::DocumentationWriter => "You are a documentation writer. Generate clear, accurate documentation including API docs, README sections, and inline comments.",
            AgentRole::Debugger => "You are a debugger. Analyze error messages, stack traces, and logs to identify root causes. Propose targeted fixes.",
            AgentRole::Architect => "You are a software architect. Design system architecture, evaluate trade-offs, define interfaces, and plan module boundaries.",
            AgentRole::PerformanceOptimizer => "You are a performance optimizer. Profile code, identify bottlenecks, suggest algorithmic improvements, and optimize memory usage.",
            AgentRole::DependencyManager => "You are a dependency manager. Audit dependencies for vulnerabilities, suggest updates, resolve conflicts, and minimize bloat.",
            AgentRole::MigrationSpecialist => "You are a migration specialist. Plan and execute data migrations, schema changes, and API version upgrades safely.",
            AgentRole::Custom(_) => "You are a specialized agent. Follow the task instructions carefully.",
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-agent definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SubAgentDef {
    pub id: String,
    pub role: AgentRole,
    pub name: String,
    pub tools: Vec<String>,
    pub max_turns: usize,
    pub context_files: Vec<String>,
    pub extra_instructions: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubAgentStatus {
    Idle,
    Running,
    Completed,
    Failed(String),
}

impl SubAgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SubAgentStatus::Idle => "idle",
            SubAgentStatus::Running => "running",
            SubAgentStatus::Completed => "completed",
            SubAgentStatus::Failed(_) => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubAgentResult {
    pub agent_id: String,
    pub role: AgentRole,
    pub status: SubAgentStatus,
    pub output: String,
    pub findings: Vec<AgentFinding>,
    pub files_modified: Vec<String>,
    pub turns_used: usize,
}

#[derive(Debug, Clone)]
pub struct AgentFinding {
    pub file: String,
    pub line: Option<usize>,
    pub severity: FindingSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindingSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl FindingSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            FindingSeverity::Error => "error",
            FindingSeverity::Warning => "warning",
            FindingSeverity::Info => "info",
            FindingSeverity::Hint => "hint",
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-agent registry
// ---------------------------------------------------------------------------

pub struct SubAgentRegistry {
    agents: Vec<SubAgentDef>,
    results: Vec<SubAgentResult>,
    agent_counter: u64,
    role_configs: HashMap<AgentRole, RoleConfig>,
}

#[derive(Debug, Clone)]
pub struct RoleConfig {
    pub default_tools: Vec<String>,
    pub max_turns: usize,
    pub auto_spawn_on: Vec<String>,
}

impl SubAgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            results: Vec::new(),
            agent_counter: 0,
            role_configs: default_role_configs(),
        }
    }

    pub fn spawn(&mut self, role: AgentRole, context_files: Vec<String>, extra_instructions: Option<String>) -> String {
        self.agent_counter += 1;
        let id = format!("subagent-{}", self.agent_counter);
        let config = self.role_configs.get(&role).cloned().unwrap_or(RoleConfig {
            default_tools: vec!["read_file".into(), "write_file".into()],
            max_turns: 10,
            auto_spawn_on: vec![],
        });
        let agent = SubAgentDef {
            id: id.clone(),
            role: role.clone(),
            name: format!("{} Agent", role.as_str()),
            tools: config.default_tools,
            max_turns: config.max_turns,
            context_files,
            extra_instructions,
        };
        self.agents.push(agent);
        id
    }

    pub fn get_agent(&self, id: &str) -> Option<&SubAgentDef> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn complete_agent(&mut self, id: &str, output: &str, findings: Vec<AgentFinding>, files_modified: Vec<String>, turns: usize) -> bool {
        if let Some(agent) = self.agents.iter().find(|a| a.id == id) {
            let result = SubAgentResult {
                agent_id: id.to_string(),
                role: agent.role.clone(),
                status: SubAgentStatus::Completed,
                output: output.to_string(),
                findings,
                files_modified,
                turns_used: turns,
            };
            self.results.push(result);
            true
        } else {
            false
        }
    }

    pub fn fail_agent(&mut self, id: &str, error: &str) -> bool {
        if let Some(agent) = self.agents.iter().find(|a| a.id == id) {
            let result = SubAgentResult {
                agent_id: id.to_string(),
                role: agent.role.clone(),
                status: SubAgentStatus::Failed(error.to_string()),
                output: String::new(),
                findings: Vec::new(),
                files_modified: Vec::new(),
                turns_used: 0,
            };
            self.results.push(result);
            true
        } else {
            false
        }
    }

    pub fn get_result(&self, id: &str) -> Option<&SubAgentResult> {
        self.results.iter().find(|r| r.agent_id == id)
    }

    pub fn results_by_role(&self, role: &AgentRole) -> Vec<&SubAgentResult> {
        self.results.iter().filter(|r| &r.role == role).collect()
    }

    pub fn all_findings(&self) -> Vec<&AgentFinding> {
        self.results.iter().flat_map(|r| r.findings.iter()).collect()
    }

    pub fn configure_role(&mut self, role: AgentRole, config: RoleConfig) {
        self.role_configs.insert(role, config);
    }

    pub fn list_agents(&self) -> &[SubAgentDef] {
        &self.agents
    }

    pub fn total_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn total_results(&self) -> usize {
        self.results.len()
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn default_role_configs() -> HashMap<AgentRole, RoleConfig> {
    let mut configs = HashMap::new();
    configs.insert(AgentRole::CodeReviewer, RoleConfig {
        default_tools: vec!["read_file".into(), "grep".into(), "glob".into()],
        max_turns: 5,
        auto_spawn_on: vec!["pull_request".into()],
    });
    configs.insert(AgentRole::TestWriter, RoleConfig {
        default_tools: vec!["read_file".into(), "write_file".into(), "run_command".into()],
        max_turns: 15,
        auto_spawn_on: vec!["new_function".into()],
    });
    configs.insert(AgentRole::SecurityReviewer, RoleConfig {
        default_tools: vec!["read_file".into(), "grep".into()],
        max_turns: 8,
        auto_spawn_on: vec!["security_sensitive_change".into()],
    });
    configs.insert(AgentRole::Debugger, RoleConfig {
        default_tools: vec!["read_file".into(), "run_command".into(), "grep".into()],
        max_turns: 20,
        auto_spawn_on: vec!["test_failure".into()],
    });
    configs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_as_str() {
        assert_eq!(AgentRole::CodeReviewer.as_str(), "code_reviewer");
        assert_eq!(AgentRole::TestWriter.as_str(), "test_writer");
        assert_eq!(AgentRole::Custom("my_role".into()).as_str(), "my_role");
    }

    #[test]
    fn test_system_prompt() {
        let prompt = AgentRole::SecurityReviewer.system_prompt();
        assert!(prompt.contains("security"));
        assert!(prompt.contains("OWASP"));
    }

    #[test]
    fn test_finding_severity() {
        assert_eq!(FindingSeverity::Error.as_str(), "error");
        assert_eq!(FindingSeverity::Hint.as_str(), "hint");
    }

    #[test]
    fn test_sub_agent_status() {
        assert_eq!(SubAgentStatus::Idle.as_str(), "idle");
        assert_eq!(SubAgentStatus::Running.as_str(), "running");
        assert_eq!(SubAgentStatus::Failed("err".into()).as_str(), "failed");
    }

    #[test]
    fn test_spawn_agent() {
        let mut reg = SubAgentRegistry::new();
        let id = reg.spawn(AgentRole::CodeReviewer, vec!["src/main.rs".into()], None);
        let agent = reg.get_agent(&id).unwrap();
        assert_eq!(agent.role, AgentRole::CodeReviewer);
        assert_eq!(agent.context_files, vec!["src/main.rs"]);
    }

    #[test]
    fn test_spawn_uses_role_config() {
        let reg = SubAgentRegistry::new();
        // CodeReviewer has max_turns=5 in default config
        let config = reg.role_configs.get(&AgentRole::CodeReviewer).unwrap();
        assert_eq!(config.max_turns, 5);
    }

    #[test]
    fn test_complete_agent() {
        let mut reg = SubAgentRegistry::new();
        let id = reg.spawn(AgentRole::TestWriter, vec![], None);
        let finding = AgentFinding {
            file: "src/lib.rs".into(),
            line: Some(42),
            severity: FindingSeverity::Warning,
            message: "Missing edge case test".into(),
            suggestion: Some("Add test for empty input".into()),
        };
        assert!(reg.complete_agent(&id, "Added 5 tests", vec![finding], vec!["src/lib_test.rs".into()], 3));
        let result = reg.get_result(&id).unwrap();
        assert_eq!(result.status, SubAgentStatus::Completed);
        assert_eq!(result.turns_used, 3);
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_fail_agent() {
        let mut reg = SubAgentRegistry::new();
        let id = reg.spawn(AgentRole::Debugger, vec![], None);
        assert!(reg.fail_agent(&id, "timeout"));
        let result = reg.get_result(&id).unwrap();
        assert_eq!(result.status, SubAgentStatus::Failed("timeout".to_string()));
    }

    #[test]
    fn test_complete_nonexistent() {
        let mut reg = SubAgentRegistry::new();
        assert!(!reg.complete_agent("fake", "", vec![], vec![], 0));
    }

    #[test]
    fn test_results_by_role() {
        let mut reg = SubAgentRegistry::new();
        let id1 = reg.spawn(AgentRole::CodeReviewer, vec![], None);
        let id2 = reg.spawn(AgentRole::TestWriter, vec![], None);
        reg.complete_agent(&id1, "ok", vec![], vec![], 1);
        reg.complete_agent(&id2, "ok", vec![], vec![], 2);
        assert_eq!(reg.results_by_role(&AgentRole::CodeReviewer).len(), 1);
        assert_eq!(reg.results_by_role(&AgentRole::TestWriter).len(), 1);
    }

    #[test]
    fn test_all_findings() {
        let mut reg = SubAgentRegistry::new();
        let id1 = reg.spawn(AgentRole::SecurityReviewer, vec![], None);
        let id2 = reg.spawn(AgentRole::CodeReviewer, vec![], None);
        let f1 = AgentFinding { file: "a.rs".into(), line: Some(1), severity: FindingSeverity::Error, message: "vuln".into(), suggestion: None };
        let f2 = AgentFinding { file: "b.rs".into(), line: Some(2), severity: FindingSeverity::Warning, message: "style".into(), suggestion: None };
        reg.complete_agent(&id1, "", vec![f1], vec![], 1);
        reg.complete_agent(&id2, "", vec![f2], vec![], 1);
        assert_eq!(reg.all_findings().len(), 2);
    }

    #[test]
    fn test_configure_role() {
        let mut reg = SubAgentRegistry::new();
        reg.configure_role(AgentRole::Architect, RoleConfig {
            default_tools: vec!["read_file".into()],
            max_turns: 30,
            auto_spawn_on: vec![],
        });
        let id = reg.spawn(AgentRole::Architect, vec![], None);
        assert_eq!(reg.get_agent(&id).unwrap().max_turns, 30);
    }

    #[test]
    fn test_spawn_custom_role() {
        let mut reg = SubAgentRegistry::new();
        let id = reg.spawn(AgentRole::Custom("api_designer".into()), vec![], Some("Design REST APIs".into()));
        let agent = reg.get_agent(&id).unwrap();
        assert_eq!(agent.role, AgentRole::Custom("api_designer".into()));
        assert_eq!(agent.extra_instructions, Some("Design REST APIs".into()));
    }

    #[test]
    fn test_list_agents() {
        let mut reg = SubAgentRegistry::new();
        reg.spawn(AgentRole::Debugger, vec![], None);
        reg.spawn(AgentRole::Refactorer, vec![], None);
        assert_eq!(reg.list_agents().len(), 2);
    }

    #[test]
    fn test_total_counts() {
        let mut reg = SubAgentRegistry::new();
        let id = reg.spawn(AgentRole::TestWriter, vec![], None);
        assert_eq!(reg.total_agents(), 1);
        assert_eq!(reg.total_results(), 0);
        reg.complete_agent(&id, "done", vec![], vec![], 1);
        assert_eq!(reg.total_results(), 1);
    }
}
