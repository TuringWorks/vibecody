
use std::time::SystemTime;

/// Named sub-agent roles for code analysis vs library analysis vs implementation,
/// inspired by Amp's Oracle/Librarian pattern.

#[derive(Debug, Clone, PartialEq)]
pub enum SubAgentRole {
    Oracle,
    Librarian,
    Implementer,
    Reviewer,
    Tester,
    Documenter,
    Architect,
    Debugger,
    Optimizer,
    SecurityExpert,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubAgentStatus {
    Idle,
    Working,
    WaitingForInput,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct SubAgent {
    pub id: String,
    pub role: SubAgentRole,
    pub name: String,
    pub status: SubAgentStatus,
    pub context: Vec<String>,
    pub capabilities: Vec<String>,
    pub system_prompt: String,
    pub task_history: Vec<SubAgentTask>,
    pub created_at: SystemTime,
    pub total_tokens_used: u64,
}

#[derive(Debug, Clone)]
pub struct SubAgentTask {
    pub id: String,
    pub description: String,
    pub input: String,
    pub output: Option<String>,
    pub status: SubAgentStatus,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub tokens_used: u64,
}

#[derive(Debug, Clone)]
pub struct SubAgentConfig {
    pub max_concurrent: usize,
    pub max_context_files: usize,
    pub default_model: String,
    pub enable_delegation: bool,
    pub task_timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct SubAgentOrchestrator {
    pub agents: Vec<SubAgent>,
    pub config: SubAgentConfig,
    pub delegation_log: Vec<Delegation>,
}

#[derive(Debug, Clone)]
pub struct Delegation {
    pub from_agent: String,
    pub to_agent: String,
    pub task_description: String,
    pub timestamp: SystemTime,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_context_files: 50,
            default_model: "claude-opus-4-6".to_string(),
            enable_delegation: true,
            task_timeout_secs: 300,
        }
    }
}

impl SubAgentRole {
    pub fn default_name(&self) -> &str {
        match self {
            SubAgentRole::Oracle => "Oracle",
            SubAgentRole::Librarian => "Librarian",
            SubAgentRole::Implementer => "Implementer",
            SubAgentRole::Reviewer => "Reviewer",
            SubAgentRole::Tester => "Tester",
            SubAgentRole::Documenter => "Documenter",
            SubAgentRole::Architect => "Architect",
            SubAgentRole::Debugger => "Debugger",
            SubAgentRole::Optimizer => "Optimizer",
            SubAgentRole::SecurityExpert => "SecurityExpert",
        }
    }

    pub fn default_capabilities(&self) -> Vec<String> {
        match self {
            SubAgentRole::Oracle => vec![
                "read_file".into(),
                "search_codebase".into(),
                "find_references".into(),
                "analyze_dependencies".into(),
                "trace_call_graph".into(),
            ],
            SubAgentRole::Librarian => vec![
                "search_docs".into(),
                "read_api_docs".into(),
                "find_examples".into(),
                "compare_libraries".into(),
                "check_versions".into(),
            ],
            SubAgentRole::Implementer => vec![
                "write_code".into(),
                "edit_file".into(),
                "create_file".into(),
                "run_build".into(),
                "apply_patch".into(),
            ],
            SubAgentRole::Reviewer => vec![
                "read_diff".into(),
                "check_style".into(),
                "find_bugs".into(),
                "suggest_improvements".into(),
                "verify_logic".into(),
            ],
            SubAgentRole::Tester => vec![
                "write_tests".into(),
                "run_tests".into(),
                "analyze_coverage".into(),
                "find_edge_cases".into(),
                "generate_fixtures".into(),
            ],
            SubAgentRole::Documenter => vec![
                "write_docs".into(),
                "generate_api_docs".into(),
                "update_readme".into(),
                "add_comments".into(),
                "create_examples".into(),
            ],
            SubAgentRole::Architect => vec![
                "design_system".into(),
                "identify_patterns".into(),
                "plan_refactor".into(),
                "evaluate_tradeoffs".into(),
                "create_diagrams".into(),
            ],
            SubAgentRole::Debugger => vec![
                "trace_execution".into(),
                "inspect_variables".into(),
                "analyze_logs".into(),
                "find_root_cause".into(),
                "reproduce_bug".into(),
            ],
            SubAgentRole::Optimizer => vec![
                "profile_code".into(),
                "find_bottlenecks".into(),
                "suggest_caching".into(),
                "reduce_allocations".into(),
                "benchmark".into(),
            ],
            SubAgentRole::SecurityExpert => vec![
                "owasp_check".into(),
                "scan_vulnerabilities".into(),
                "review_auth".into(),
                "check_inputs".into(),
                "audit_dependencies".into(),
            ],
        }
    }

    pub fn system_prompt(&self) -> String {
        match self {
            SubAgentRole::Oracle => {
                "You are Oracle, a code analysis expert. Your job is to read and understand \
                 codebases deeply. You find patterns, trace call graphs, answer questions about \
                 how code works, and identify relationships between components. You never modify \
                 code—only analyze and explain."
                    .to_string()
            }
            SubAgentRole::Librarian => {
                "You are Librarian, an external library and documentation expert. You research \
                 APIs, find best practices, compare library options, and provide usage examples. \
                 You stay current on library versions and breaking changes."
                    .to_string()
            }
            SubAgentRole::Implementer => {
                "You are Implementer, a code writing specialist. You implement features, fix \
                 bugs, and write clean, well-structured code. You follow the project's existing \
                 conventions and patterns. You write production-quality code with proper error \
                 handling."
                    .to_string()
            }
            SubAgentRole::Reviewer => {
                "You are Reviewer, a code review expert. You examine changes for correctness, \
                 style consistency, potential bugs, and improvement opportunities. You provide \
                 constructive, actionable feedback."
                    .to_string()
            }
            SubAgentRole::Tester => {
                "You are Tester, a test generation specialist. You write comprehensive tests \
                 including unit tests, integration tests, and edge cases. You identify untested \
                 paths and ensure thorough coverage."
                    .to_string()
            }
            SubAgentRole::Documenter => {
                "You are Documenter, a documentation specialist. You generate clear, accurate \
                 documentation including API docs, READMEs, inline comments, and usage examples. \
                 You write for the target audience."
                    .to_string()
            }
            SubAgentRole::Architect => {
                "You are Architect, a system design expert. You plan system structure, identify \
                 design patterns, evaluate tradeoffs, and propose clean architectures. You think \
                 about scalability, maintainability, and separation of concerns."
                    .to_string()
            }
            SubAgentRole::Debugger => {
                "You are Debugger, a debugging specialist. You trace execution paths, identify \
                 root causes, analyze error logs, and help reproduce issues. You think \
                 systematically about what could go wrong."
                    .to_string()
            }
            SubAgentRole::Optimizer => {
                "You are Optimizer, a performance specialist. You identify bottlenecks, suggest \
                 caching strategies, reduce allocations, and improve algorithmic complexity. You \
                 measure before and after."
                    .to_string()
            }
            SubAgentRole::SecurityExpert => {
                "You are SecurityExpert, a security specialist. You perform OWASP checks, detect \
                 vulnerabilities, review authentication flows, validate input handling, and audit \
                 dependencies for known CVEs."
                    .to_string()
            }
        }
    }

    pub fn suggested_model(&self) -> &str {
        match self {
            SubAgentRole::Oracle => "claude-opus-4-6",
            SubAgentRole::Librarian => "claude-sonnet-4-20250514",
            SubAgentRole::Implementer => "claude-opus-4-6",
            SubAgentRole::Reviewer => "claude-opus-4-6",
            SubAgentRole::Tester => "claude-sonnet-4-20250514",
            SubAgentRole::Documenter => "claude-sonnet-4-20250514",
            SubAgentRole::Architect => "claude-opus-4-6",
            SubAgentRole::Debugger => "claude-opus-4-6",
            SubAgentRole::Optimizer => "claude-sonnet-4-20250514",
            SubAgentRole::SecurityExpert => "claude-opus-4-6",
        }
    }

    fn all_variants() -> &'static [SubAgentRole] {
        &[
            SubAgentRole::Oracle,
            SubAgentRole::Librarian,
            SubAgentRole::Implementer,
            SubAgentRole::Reviewer,
            SubAgentRole::Tester,
            SubAgentRole::Documenter,
            SubAgentRole::Architect,
            SubAgentRole::Debugger,
            SubAgentRole::Optimizer,
            SubAgentRole::SecurityExpert,
        ]
    }
}

impl SubAgent {
    pub fn new(role: SubAgentRole) -> Self {
        let name = role.default_name().to_string();
        let capabilities = role.default_capabilities();
        let system_prompt = role.system_prompt();
        let now = SystemTime::now();
        let id = format!(
            "{}-{}",
            role.default_name().to_lowercase(),
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                % 1_000_000
        );
        Self {
            id,
            role,
            name,
            status: SubAgentStatus::Idle,
            context: Vec::new(),
            capabilities,
            system_prompt,
            task_history: Vec::new(),
            created_at: now,
            total_tokens_used: 0,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn assign_task(&mut self, description: &str, input: &str) -> &SubAgentTask {
        let task_id = format!("task-{}", self.task_history.len() + 1);
        let task = SubAgentTask {
            id: task_id,
            description: description.to_string(),
            input: input.to_string(),
            output: None,
            status: SubAgentStatus::Working,
            started_at: SystemTime::now(),
            completed_at: None,
            tokens_used: 0,
        };
        self.status = SubAgentStatus::Working;
        self.task_history.push(task);
        self.task_history.last().expect("just pushed a task")
    }

    pub fn complete_task(&mut self, task_id: &str, output: &str) {
        if let Some(task) = self.task_history.iter_mut().find(|t| t.id == task_id) {
            task.output = Some(output.to_string());
            task.status = SubAgentStatus::Completed;
            task.completed_at = Some(SystemTime::now());
        }
        // If no more working tasks, set agent to idle
        let any_working = self
            .task_history
            .iter()
            .any(|t| t.status == SubAgentStatus::Working);
        if !any_working {
            self.status = SubAgentStatus::Idle;
        }
    }

    pub fn fail_task(&mut self, task_id: &str, error: &str) {
        if let Some(task) = self.task_history.iter_mut().find(|t| t.id == task_id) {
            task.status = SubAgentStatus::Failed(error.to_string());
            task.completed_at = Some(SystemTime::now());
        }
        let any_working = self
            .task_history
            .iter()
            .any(|t| t.status == SubAgentStatus::Working);
        if !any_working {
            self.status = SubAgentStatus::Idle;
        }
    }

    pub fn add_context(&mut self, file_or_doc: &str) {
        if !self.context.contains(&file_or_doc.to_string()) {
            self.context.push(file_or_doc.to_string());
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self.status, SubAgentStatus::Idle)
    }

    pub fn total_tasks(&self) -> usize {
        self.task_history.len()
    }

    pub fn success_rate(&self) -> f64 {
        let completed_or_failed: Vec<&SubAgentTask> = self
            .task_history
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    SubAgentStatus::Completed | SubAgentStatus::Failed(_)
                )
            })
            .collect();
        if completed_or_failed.is_empty() {
            return 0.0;
        }
        let successes = completed_or_failed
            .iter()
            .filter(|t| t.status == SubAgentStatus::Completed)
            .count();
        successes as f64 / completed_or_failed.len() as f64
    }
}

impl SubAgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            config: SubAgentConfig::default(),
            delegation_log: Vec::new(),
        }
    }

    pub fn with_config(config: SubAgentConfig) -> Self {
        Self {
            agents: Vec::new(),
            config,
            delegation_log: Vec::new(),
        }
    }

    pub fn spawn(&mut self, role: SubAgentRole) -> &SubAgent {
        let agent = SubAgent::new(role);
        self.agents.push(agent);
        self.agents.last().expect("just pushed an agent")
    }

    pub fn spawn_team(&mut self, roles: Vec<SubAgentRole>) -> Vec<String> {
        let mut ids = Vec::new();
        for role in roles {
            let agent = SubAgent::new(role);
            ids.push(agent.id.clone());
            self.agents.push(agent);
        }
        ids
    }

    pub fn get_agent(&self, id: &str) -> Option<&SubAgent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn get_agent_mut(&mut self, id: &str) -> Option<&mut SubAgent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    pub fn delegate(
        &mut self,
        from_id: &str,
        to_id: &str,
        task_description: &str,
    ) -> Result<(), String> {
        if !self.config.enable_delegation {
            return Err("Delegation is disabled".to_string());
        }
        // Verify both agents exist
        let from_exists = self.agents.iter().any(|a| a.id == from_id);
        let to_exists = self.agents.iter().any(|a| a.id == to_id);
        if !from_exists {
            return Err(format!("Source agent '{}' not found", from_id));
        }
        if !to_exists {
            return Err(format!("Target agent '{}' not found", to_id));
        }
        // Check target is available
        let to_available = self
            .agents
            .iter()
            .find(|a| a.id == to_id)
            .map(|a| a.is_available())
            .unwrap_or(false);
        if !to_available {
            return Err(format!("Target agent '{}' is not available", to_id));
        }

        self.delegation_log.push(Delegation {
            from_agent: from_id.to_string(),
            to_agent: to_id.to_string(),
            task_description: task_description.to_string(),
            timestamp: SystemTime::now(),
        });

        // Assign task on target agent
        if let Some(agent) = self.agents.iter_mut().find(|a| a.id == to_id) {
            agent.assign_task(task_description, &format!("Delegated from {}", from_id));
        }

        Ok(())
    }

    pub fn available_agents(&self) -> Vec<&SubAgent> {
        self.agents.iter().filter(|a| a.is_available()).collect()
    }

    pub fn find_best_agent_for(&self, task_description: &str) -> Option<&SubAgent> {
        let lower = task_description.to_lowercase();
        let keyword_role_map: Vec<(&[&str], &SubAgentRole)> = vec![
            (
                &["analyze", "find", "search", "pattern", "understand", "read"],
                &SubAgentRole::Oracle,
            ),
            (
                &["library", "docs", "api", "dependency", "package", "crate"],
                &SubAgentRole::Librarian,
            ),
            (
                &["implement", "write", "create", "build", "code", "feature"],
                &SubAgentRole::Implementer,
            ),
            (
                &["review", "check", "style", "quality", "pr"],
                &SubAgentRole::Reviewer,
            ),
            (
                &["test", "coverage", "assert", "spec", "edge case"],
                &SubAgentRole::Tester,
            ),
            (
                &["document", "readme", "doc", "comment", "explain"],
                &SubAgentRole::Documenter,
            ),
            (
                &["design", "architect", "structure", "plan", "refactor"],
                &SubAgentRole::Architect,
            ),
            (
                &["debug", "trace", "error", "bug", "crash", "fix"],
                &SubAgentRole::Debugger,
            ),
            (
                &[
                    "optimize",
                    "performance",
                    "speed",
                    "bottleneck",
                    "memory",
                    "slow",
                ],
                &SubAgentRole::Optimizer,
            ),
            (
                &[
                    "security",
                    "vulnerability",
                    "owasp",
                    "auth",
                    "cve",
                    "injection",
                ],
                &SubAgentRole::SecurityExpert,
            ),
        ];

        let mut best_role: Option<&SubAgentRole> = None;
        let mut best_score = 0usize;

        for (keywords, role) in &keyword_role_map {
            let score = keywords.iter().filter(|kw| lower.contains(**kw)).count();
            if score > best_score {
                best_score = score;
                best_role = Some(role);
            }
        }

        if let Some(role) = best_role {
            self.agents
                .iter()
                .filter(|a| a.role == *role && a.is_available())
                .next()
                .or_else(|| {
                    // Fall back to any available agent with matching role
                    self.agents.iter().filter(|a| a.role == *role).next()
                })
        } else {
            // Return first available agent
            self.available_agents().into_iter().next()
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.agents.iter().map(|a| a.total_tokens_used).sum()
    }

    pub fn dismiss(&mut self, id: &str) -> bool {
        let len_before = self.agents.len();
        self.agents.retain(|a| a.id != id);
        self.agents.len() < len_before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_default_name() {
        assert_eq!(SubAgentRole::Oracle.default_name(), "Oracle");
        assert_eq!(SubAgentRole::Librarian.default_name(), "Librarian");
        assert_eq!(SubAgentRole::Implementer.default_name(), "Implementer");
        assert_eq!(SubAgentRole::SecurityExpert.default_name(), "SecurityExpert");
    }

    #[test]
    fn test_role_default_capabilities_non_empty() {
        for role in SubAgentRole::all_variants() {
            let caps = role.default_capabilities();
            assert!(!caps.is_empty(), "{:?} should have capabilities", role);
            assert!(caps.len() >= 4, "{:?} should have at least 4 capabilities", role);
        }
    }

    #[test]
    fn test_role_system_prompt_non_empty() {
        for role in SubAgentRole::all_variants() {
            let prompt = role.system_prompt();
            assert!(!prompt.is_empty());
            assert!(prompt.len() > 50, "Prompt for {:?} too short", role);
        }
    }

    #[test]
    fn test_role_suggested_model() {
        assert_eq!(SubAgentRole::Oracle.suggested_model(), "claude-opus-4-6");
        assert_eq!(
            SubAgentRole::Tester.suggested_model(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(SubAgentRole::Implementer.suggested_model(), "claude-opus-4-6");
    }

    #[test]
    fn test_sub_agent_new() {
        let agent = SubAgent::new(SubAgentRole::Oracle);
        assert_eq!(agent.name, "Oracle");
        assert_eq!(agent.role, SubAgentRole::Oracle);
        assert!(agent.is_available());
        assert!(agent.id.starts_with("oracle-"));
        assert!(agent.task_history.is_empty());
    }

    #[test]
    fn test_sub_agent_with_name() {
        let agent = SubAgent::new(SubAgentRole::Implementer).with_name("CodeBot");
        assert_eq!(agent.name, "CodeBot");
        assert_eq!(agent.role, SubAgentRole::Implementer);
    }

    #[test]
    fn test_assign_task() {
        let mut agent = SubAgent::new(SubAgentRole::Tester);
        let task = agent.assign_task("Write unit tests", "for module foo");
        assert_eq!(task.description, "Write unit tests");
        assert_eq!(task.input, "for module foo");
        assert_eq!(task.status, SubAgentStatus::Working);
        assert!(!agent.is_available());
        assert_eq!(agent.total_tasks(), 1);
    }

    #[test]
    fn test_complete_task() {
        let mut agent = SubAgent::new(SubAgentRole::Oracle);
        agent.assign_task("Analyze code", "src/main.rs");
        let task_id = agent.task_history[0].id.clone();
        agent.complete_task(&task_id, "Found 3 patterns");
        assert!(agent.is_available());
        assert_eq!(
            agent.task_history[0].output,
            Some("Found 3 patterns".to_string())
        );
        assert_eq!(agent.task_history[0].status, SubAgentStatus::Completed);
        assert!(agent.task_history[0].completed_at.is_some());
    }

    #[test]
    fn test_fail_task() {
        let mut agent = SubAgent::new(SubAgentRole::Debugger);
        agent.assign_task("Debug crash", "stack trace");
        let task_id = agent.task_history[0].id.clone();
        agent.fail_task(&task_id, "Could not reproduce");
        assert!(agent.is_available());
        assert_eq!(
            agent.task_history[0].status,
            SubAgentStatus::Failed("Could not reproduce".to_string())
        );
    }

    #[test]
    fn test_add_context_dedup() {
        let mut agent = SubAgent::new(SubAgentRole::Oracle);
        agent.add_context("src/main.rs");
        agent.add_context("src/lib.rs");
        agent.add_context("src/main.rs"); // duplicate
        assert_eq!(agent.context.len(), 2);
    }

    #[test]
    fn test_success_rate_empty() {
        let agent = SubAgent::new(SubAgentRole::Tester);
        assert_eq!(agent.success_rate(), 0.0);
    }

    #[test]
    fn test_success_rate_all_success() {
        let mut agent = SubAgent::new(SubAgentRole::Tester);
        agent.assign_task("t1", "i1");
        let id = agent.task_history[0].id.clone();
        agent.complete_task(&id, "done");
        agent.assign_task("t2", "i2");
        let id2 = agent.task_history[1].id.clone();
        agent.complete_task(&id2, "done2");
        assert_eq!(agent.success_rate(), 1.0);
    }

    #[test]
    fn test_success_rate_mixed() {
        let mut agent = SubAgent::new(SubAgentRole::Tester);
        agent.assign_task("t1", "i1");
        let id = agent.task_history[0].id.clone();
        agent.complete_task(&id, "done");
        agent.assign_task("t2", "i2");
        let id2 = agent.task_history[1].id.clone();
        agent.fail_task(&id2, "err");
        assert!((agent.success_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_orchestrator_new() {
        let orch = SubAgentOrchestrator::new();
        assert!(orch.agents.is_empty());
        assert_eq!(orch.config.max_concurrent, 5);
        assert!(orch.config.enable_delegation);
    }

    #[test]
    fn test_orchestrator_spawn() {
        let mut orch = SubAgentOrchestrator::new();
        let agent = orch.spawn(SubAgentRole::Oracle);
        assert_eq!(agent.role, SubAgentRole::Oracle);
        assert_eq!(orch.agents.len(), 1);
    }

    #[test]
    fn test_orchestrator_spawn_team() {
        let mut orch = SubAgentOrchestrator::new();
        let ids = orch.spawn_team(vec![
            SubAgentRole::Oracle,
            SubAgentRole::Implementer,
            SubAgentRole::Tester,
        ]);
        assert_eq!(ids.len(), 3);
        assert_eq!(orch.agents.len(), 3);
    }

    #[test]
    fn test_orchestrator_get_agent() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        let id = orch.agents[0].id.clone();
        assert!(orch.get_agent(&id).is_some());
        assert!(orch.get_agent("nonexistent").is_none());
    }

    #[test]
    fn test_orchestrator_delegate() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Implementer);
        let from_id = orch.agents[0].id.clone();
        let to_id = orch.agents[1].id.clone();
        let result = orch.delegate(&from_id, &to_id, "Implement feature X");
        assert!(result.is_ok());
        assert_eq!(orch.delegation_log.len(), 1);
        assert_eq!(orch.delegation_log[0].task_description, "Implement feature X");
    }

    #[test]
    fn test_orchestrator_delegate_disabled() {
        let mut config = SubAgentConfig::default();
        config.enable_delegation = false;
        let mut orch = SubAgentOrchestrator::with_config(config);
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Implementer);
        let from_id = orch.agents[0].id.clone();
        let to_id = orch.agents[1].id.clone();
        let result = orch.delegate(&from_id, &to_id, "task");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disabled"));
    }

    #[test]
    fn test_orchestrator_delegate_nonexistent_source() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Implementer);
        let to_id = orch.agents[0].id.clone();
        let result = orch.delegate("nonexistent", &to_id, "task");
        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_delegate_nonexistent_target() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        let from_id = orch.agents[0].id.clone();
        let result = orch.delegate(&from_id, "nonexistent", "task");
        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_delegate_busy_target() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Implementer);
        let from_id = orch.agents[0].id.clone();
        let to_id = orch.agents[1].id.clone();
        // Make target busy
        orch.agents[1].assign_task("busy", "input");
        let result = orch.delegate(&from_id, &to_id, "new task");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not available"));
    }

    #[test]
    fn test_orchestrator_available_agents() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Tester);
        assert_eq!(orch.available_agents().len(), 2);
        orch.agents[0].assign_task("work", "data");
        assert_eq!(orch.available_agents().len(), 1);
    }

    #[test]
    fn test_orchestrator_find_best_agent_for_test() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Tester);
        orch.spawn(SubAgentRole::Implementer);
        let best = orch.find_best_agent_for("write unit tests for coverage");
        assert!(best.is_some());
        assert_eq!(best.unwrap().role, SubAgentRole::Tester);
    }

    #[test]
    fn test_orchestrator_find_best_agent_for_security() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::SecurityExpert);
        orch.spawn(SubAgentRole::Oracle);
        let best = orch.find_best_agent_for("check for OWASP vulnerabilities and auth issues");
        assert!(best.is_some());
        assert_eq!(best.unwrap().role, SubAgentRole::SecurityExpert);
    }

    #[test]
    fn test_orchestrator_find_best_agent_no_match() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        let best = orch.find_best_agent_for("something completely unrelated xyzzy");
        // Falls back to first available
        assert!(best.is_some());
    }

    #[test]
    fn test_orchestrator_total_tokens() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        orch.spawn(SubAgentRole::Tester);
        orch.agents[0].total_tokens_used = 1000;
        orch.agents[1].total_tokens_used = 2000;
        assert_eq!(orch.total_tokens(), 3000);
    }

    #[test]
    fn test_orchestrator_dismiss() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Oracle);
        let id = orch.agents[0].id.clone();
        assert!(orch.dismiss(&id));
        assert!(orch.agents.is_empty());
    }

    #[test]
    fn test_orchestrator_dismiss_nonexistent() {
        let mut orch = SubAgentOrchestrator::new();
        assert!(!orch.dismiss("nonexistent"));
    }

    #[test]
    fn test_config_default() {
        let config = SubAgentConfig::default();
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.max_context_files, 50);
        assert_eq!(config.default_model, "claude-opus-4-6");
        assert!(config.enable_delegation);
        assert_eq!(config.task_timeout_secs, 300);
    }

    #[test]
    fn test_multiple_tasks_on_agent() {
        let mut agent = SubAgent::new(SubAgentRole::Implementer);
        agent.assign_task("task1", "input1");
        let id1 = agent.task_history[0].id.clone();
        agent.complete_task(&id1, "done1");
        agent.assign_task("task2", "input2");
        assert_eq!(agent.total_tasks(), 2);
        assert!(!agent.is_available()); // second task still working
    }

    #[test]
    fn test_complete_nonexistent_task() {
        let mut agent = SubAgent::new(SubAgentRole::Oracle);
        agent.assign_task("task1", "input1");
        agent.complete_task("nonexistent", "output");
        // Original task should still be working
        assert_eq!(agent.task_history[0].status, SubAgentStatus::Working);
    }

    #[test]
    fn test_status_equality() {
        assert_eq!(SubAgentStatus::Idle, SubAgentStatus::Idle);
        assert_eq!(SubAgentStatus::Working, SubAgentStatus::Working);
        assert_ne!(SubAgentStatus::Idle, SubAgentStatus::Working);
        assert_eq!(
            SubAgentStatus::Failed("err".into()),
            SubAgentStatus::Failed("err".into())
        );
    }

    #[test]
    fn test_orchestrator_find_best_for_optimization() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Optimizer);
        orch.spawn(SubAgentRole::Oracle);
        let best = orch.find_best_agent_for("optimize performance and find bottleneck");
        assert!(best.is_some());
        assert_eq!(best.unwrap().role, SubAgentRole::Optimizer);
    }

    #[test]
    fn test_orchestrator_find_best_for_docs() {
        let mut orch = SubAgentOrchestrator::new();
        orch.spawn(SubAgentRole::Documenter);
        orch.spawn(SubAgentRole::Oracle);
        let best = orch.find_best_agent_for("document and explain the module");
        assert!(best.is_some());
        assert_eq!(best.unwrap().role, SubAgentRole::Documenter);
    }
}
