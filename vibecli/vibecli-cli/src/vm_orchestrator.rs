//! Cloud VM agent orchestration for VibeCody.
//!
//! Enables Cursor-style "8 parallel agents in isolated VMs" workflow.
//! Each task runs in its own container/VM with resource limits, automatic
//! branch creation, and PR generation on completion.
//!
//! REPL commands: `/vm-orchestrator launch|list|status|stop|cleanup|conflicts|resources`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeType {
    Docker,
    Podman,
    CloudVm,
}

impl std::fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::Podman => write!(f, "podman"),
            Self::CloudVm => write!(f, "cloud-vm"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvStatus {
    Provisioning,
    Cloning,
    Working,
    Committing,
    CreatingPR,
    Completed,
    Failed,
    Cleaning,
}

impl std::fmt::Display for EnvStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provisioning => write!(f, "provisioning"),
            Self::Cloning => write!(f, "cloning"),
            Self::Working => write!(f, "working"),
            Self::Committing => write!(f, "committing"),
            Self::CreatingPR => write!(f, "creating-pr"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cleaning => write!(f, "cleaning"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorError {
    MaxEnvironmentsReached,
    EnvironmentNotFound,
    ProvisionFailed(String),
    CloneFailed(String),
    BranchConflict(String),
    PRCreationFailed(String),
    ResourceExhausted(String),
    TimeoutExceeded(u64),
    CleanupFailed(String),
    InvalidConfig(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxEnvironmentsReached => write!(f, "maximum parallel environments reached"),
            Self::EnvironmentNotFound => write!(f, "environment not found"),
            Self::ProvisionFailed(msg) => write!(f, "provision failed: {msg}"),
            Self::CloneFailed(msg) => write!(f, "clone failed: {msg}"),
            Self::BranchConflict(msg) => write!(f, "branch conflict: {msg}"),
            Self::PRCreationFailed(msg) => write!(f, "PR creation failed: {msg}"),
            Self::ResourceExhausted(msg) => write!(f, "resource exhausted: {msg}"),
            Self::TimeoutExceeded(secs) => write!(f, "timeout exceeded: {secs}s"),
            Self::CleanupFailed(msg) => write!(f, "cleanup failed: {msg}"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
        }
    }
}

// === Structs ===

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub timeout_secs: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_cores: 2.0,
            memory_mb: 4096,
            disk_mb: 10240,
            timeout_secs: 3600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_parallel_envs: usize,
    pub runtime: RuntimeType,
    pub default_image: String,
    pub resource_limits: ResourceLimits,
    pub git_remote: Option<String>,
    pub auto_pr: bool,
    pub cleanup_on_complete: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_parallel_envs: 8,
            runtime: RuntimeType::Docker,
            default_image: "vibecody/agent-env:latest".to_string(),
            resource_limits: ResourceLimits::default(),
            git_remote: None,
            auto_pr: true,
            cleanup_on_complete: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub duration_secs: u64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            disk_mb: 0,
            duration_secs: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentEnvironment {
    pub id: String,
    pub task_description: String,
    pub branch_name: String,
    pub status: EnvStatus,
    pub container_id: Option<String>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub pr_url: Option<String>,
    pub resource_usage: ResourceUsage,
    pub log_lines: Vec<String>,
    pub error: Option<String>,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequestSpec {
    pub title: String,
    pub description: String,
    pub branch: String,
    pub base_branch: String,
    pub files_changed: Vec<String>,
    pub test_plan: String,
    pub agent_trace_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub branch_a: String,
    pub branch_b: String,
    pub conflicting_files: Vec<String>,
    pub detected_at: u64,
    pub resolution_suggestion: String,
}

// === VmOrchestrator ===

#[derive(Debug)]
pub struct VmOrchestrator {
    pub config: OrchestratorConfig,
    environments: HashMap<String, AgentEnvironment>,
    next_id: u64,
}

impl VmOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            environments: HashMap::new(),
            next_id: 1,
        }
    }

    /// Launch a new agent environment for the given task.
    pub fn launch_environment(&mut self, task: &str) -> Result<String, OrchestratorError> {
        let active = self.active_count();
        if active >= self.config.max_parallel_envs {
            return Err(OrchestratorError::MaxEnvironmentsReached);
        }

        let id = format!("env-{:06}", self.next_id);
        self.next_id += 1;

        let branch_name = self.generate_branch_name(task);

        let env = AgentEnvironment {
            id: id.clone(),
            task_description: task.to_string(),
            branch_name,
            status: EnvStatus::Provisioning,
            container_id: None,
            created_at: current_timestamp(),
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };

        self.environments.insert(id.clone(), env);
        Ok(id)
    }

    /// Get a reference to an environment by ID.
    pub fn get_environment(&self, id: &str) -> Option<&AgentEnvironment> {
        self.environments.get(id)
    }

    /// List all environments.
    pub fn list_environments(&self) -> Vec<&AgentEnvironment> {
        let mut envs: Vec<&AgentEnvironment> = self.environments.values().collect();
        envs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        envs
    }

    /// Count active (non-terminal) environments.
    pub fn active_count(&self) -> usize {
        self.environments
            .values()
            .filter(|e| !matches!(e.status, EnvStatus::Completed | EnvStatus::Failed | EnvStatus::Cleaning))
            .count()
    }

    /// Transition an environment to a new status.
    pub fn transition_status(
        &mut self,
        id: &str,
        new_status: EnvStatus,
    ) -> Result<(), OrchestratorError> {
        let env = self
            .environments
            .get_mut(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;
        env.status = new_status;
        Ok(())
    }

    /// Stop an environment (mark as failed).
    pub fn stop_environment(&mut self, id: &str) -> Result<(), OrchestratorError> {
        let env = self
            .environments
            .get_mut(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;
        env.status = EnvStatus::Failed;
        env.error = Some("stopped by user".to_string());
        Ok(())
    }

    /// Cleanup an environment (set to Cleaning then remove).
    pub fn cleanup_environment(&mut self, id: &str) -> Result<(), OrchestratorError> {
        if !self.environments.contains_key(id) {
            return Err(OrchestratorError::EnvironmentNotFound);
        }
        // Set to Cleaning briefly, then remove
        if let Some(env) = self.environments.get_mut(id) {
            env.status = EnvStatus::Cleaning;
        }
        self.environments.remove(id);
        Ok(())
    }

    /// Generate a branch name from a task description.
    pub fn generate_branch_name(&self, task: &str) -> String {
        let slug: String = task
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-");

        let slug = if slug.len() > 40 {
            slug[..40].trim_end_matches('-').to_string()
        } else {
            slug
        };

        let short_id = format!("{:06x}", self.next_id);
        let short_id = &short_id[short_id.len().saturating_sub(6)..];
        format!("agent/{}-{}", slug, short_id)
    }

    /// Generate the docker run command for an environment.
    pub fn generate_docker_run_cmd(&self, env: &AgentEnvironment) -> String {
        let runtime = match &self.config.runtime {
            RuntimeType::Docker => "docker",
            RuntimeType::Podman => "podman",
            RuntimeType::CloudVm => "docker", // fallback
        };

        let limits = &self.config.resource_limits;
        let mut parts = vec![
            runtime.to_string(),
            "run".to_string(),
            "--rm".to_string(),
            "-d".to_string(),
            format!("--name={}", env.id),
            format!("--cpus={}", limits.cpu_cores),
            format!("--memory={}m", limits.memory_mb),
            format!("--storage-opt=size={}m", limits.disk_mb),
            format!("-e=BRANCH={}", env.branch_name),
            format!("-e=TASK_ID={}", env.id),
        ];

        if let Some(ref remote) = self.config.git_remote {
            parts.push(format!("-e=GIT_REMOTE={}", remote));
        }

        parts.push("-v=/workspace:/workspace".to_string());
        parts.push(self.config.default_image.clone());

        parts.join(" ")
    }

    /// Complete an environment and generate a PR spec.
    pub fn complete_environment(
        &mut self,
        id: &str,
        files_changed: Vec<String>,
    ) -> Result<PullRequestSpec, OrchestratorError> {
        let env = self
            .environments
            .get_mut(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;

        env.status = EnvStatus::Completed;
        env.completed_at = Some(current_timestamp());
        env.files_changed = files_changed;

        let spec = self.generate_pr_spec(
            &self.environments[id].clone(),
        );

        Ok(spec)
    }

    /// Fail an environment with an error message.
    pub fn fail_environment(
        &mut self,
        id: &str,
        error: &str,
    ) -> Result<(), OrchestratorError> {
        let env = self
            .environments
            .get_mut(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;
        env.status = EnvStatus::Failed;
        env.error = Some(error.to_string());
        Ok(())
    }

    /// Generate a PR spec from an environment.
    pub fn generate_pr_spec(&self, env: &AgentEnvironment) -> PullRequestSpec {
        let title = if env.task_description.len() > 60 {
            format!("agent: {}", &env.task_description[..57].trim_end())
        } else {
            format!("agent: {}", &env.task_description)
        };

        let file_list = if env.files_changed.is_empty() {
            "No files changed.".to_string()
        } else {
            env.files_changed
                .iter()
                .map(|f| format!("- `{}`", f))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let description = format!(
            "## Summary\n\
             Automated agent task: {}\n\n\
             ## Files Changed\n\
             {}\n\n\
             ## Agent Environment\n\
             - Environment ID: `{}`\n\
             - Branch: `{}`\n\
             - Duration: {}s",
            env.task_description,
            file_list,
            env.id,
            env.branch_name,
            env.resource_usage.duration_secs,
        );

        let test_plan = format!(
            "- [ ] Verify {} files changed compile correctly\n\
             - [ ] Run unit tests on affected modules\n\
             - [ ] Review agent trace for correctness\n\
             - [ ] Check for unintended side effects",
            env.files_changed.len(),
        );

        PullRequestSpec {
            title,
            description,
            branch: env.branch_name.clone(),
            base_branch: "main".to_string(),
            files_changed: env.files_changed.clone(),
            test_plan,
            agent_trace_url: None,
        }
    }

    /// Detect potential conflicts between active environments.
    pub fn detect_conflicts(&self) -> Vec<ConflictInfo> {
        let active: Vec<&AgentEnvironment> = self
            .environments
            .values()
            .filter(|e| {
                matches!(
                    e.status,
                    EnvStatus::Working | EnvStatus::Committing | EnvStatus::CreatingPR
                )
            })
            .collect();

        let mut conflicts = Vec::new();

        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                let a = active[i];
                let b = active[j];

                let overlapping: Vec<String> = a
                    .files_changed
                    .iter()
                    .filter(|f| b.files_changed.contains(f))
                    .cloned()
                    .collect();

                if !overlapping.is_empty() {
                    let suggestion = if overlapping.len() == 1 {
                        format!(
                            "Single file overlap ({}); consider sequencing these tasks",
                            overlapping[0]
                        )
                    } else {
                        format!(
                            "{} files overlap; consider merging {} first to avoid conflicts",
                            overlapping.len(),
                            a.branch_name
                        )
                    };

                    conflicts.push(ConflictInfo {
                        branch_a: a.branch_name.clone(),
                        branch_b: b.branch_name.clone(),
                        conflicting_files: overlapping,
                        detected_at: current_timestamp(),
                        resolution_suggestion: suggestion,
                    });
                }
            }
        }

        conflicts
    }

    /// Get total resource usage across all active environments.
    pub fn get_total_resource_usage(&self) -> ResourceUsage {
        let mut total = ResourceUsage::default();
        for env in self.environments.values() {
            if matches!(
                env.status,
                EnvStatus::Provisioning
                    | EnvStatus::Cloning
                    | EnvStatus::Working
                    | EnvStatus::Committing
                    | EnvStatus::CreatingPR
            ) {
                total.cpu_percent += env.resource_usage.cpu_percent;
                total.memory_mb += env.resource_usage.memory_mb;
                total.disk_mb += env.resource_usage.disk_mb;
                total.duration_secs = total
                    .duration_secs
                    .max(env.resource_usage.duration_secs);
            }
        }
        total
    }

    /// Add a log line to an environment.
    pub fn add_log(&mut self, id: &str, line: &str) -> Result<(), OrchestratorError> {
        let env = self
            .environments
            .get_mut(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;
        env.log_lines.push(line.to_string());
        Ok(())
    }

    /// Get log lines for an environment.
    pub fn get_logs(&self, id: &str) -> Result<Vec<String>, OrchestratorError> {
        let env = self
            .environments
            .get(id)
            .ok_or(OrchestratorError::EnvironmentNotFound)?;
        Ok(env.log_lines.clone())
    }
}

/// Simple timestamp helper (seconds since an epoch-like counter for determinism in tests).
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_orchestrator() -> VmOrchestrator {
        VmOrchestrator::new(OrchestratorConfig::default())
    }

    // --- Config defaults ---

    #[test]
    fn test_config_defaults() {
        let cfg = OrchestratorConfig::default();
        assert_eq!(cfg.max_parallel_envs, 8);
        assert_eq!(cfg.runtime, RuntimeType::Docker);
        assert_eq!(cfg.default_image, "vibecody/agent-env:latest");
        assert!(cfg.auto_pr);
        assert!(cfg.cleanup_on_complete);
        assert!(cfg.git_remote.is_none());
    }

    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();
        assert!((limits.cpu_cores - 2.0).abs() < f32::EPSILON);
        assert_eq!(limits.memory_mb, 4096);
        assert_eq!(limits.disk_mb, 10240);
        assert_eq!(limits.timeout_secs, 3600);
    }

    #[test]
    fn test_resource_usage_defaults() {
        let usage = ResourceUsage::default();
        assert!((usage.cpu_percent - 0.0).abs() < f32::EPSILON);
        assert_eq!(usage.memory_mb, 0);
        assert_eq!(usage.disk_mb, 0);
        assert_eq!(usage.duration_secs, 0);
    }

    // --- Launch environment ---

    #[test]
    fn test_launch_environment_success() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("Fix login bug").unwrap();
        assert!(id.starts_with("env-"));
        assert_eq!(orch.environments.len(), 1);
        let env = orch.get_environment(&id).unwrap();
        assert_eq!(env.status, EnvStatus::Provisioning);
        assert_eq!(env.task_description, "Fix login bug");
        assert!(env.branch_name.starts_with("agent/"));
    }

    #[test]
    fn test_launch_environment_max_reached() {
        let mut config = OrchestratorConfig::default();
        config.max_parallel_envs = 2;
        let mut orch = VmOrchestrator::new(config);
        orch.launch_environment("task 1").unwrap();
        orch.launch_environment("task 2").unwrap();
        let result = orch.launch_environment("task 3");
        assert_eq!(result.unwrap_err(), OrchestratorError::MaxEnvironmentsReached);
    }

    #[test]
    fn test_launch_increments_id() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        assert_ne!(id1, id2);
        assert_eq!(id1, "env-000001");
        assert_eq!(id2, "env-000002");
    }

    #[test]
    fn test_launch_sets_created_at() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        let env = orch.get_environment(&id).unwrap();
        assert!(env.created_at > 0);
    }

    // --- Branch name generation ---

    #[test]
    fn test_branch_name_basic() {
        let orch = default_orchestrator();
        let name = orch.generate_branch_name("Fix login bug");
        assert!(name.starts_with("agent/fix-login-bug-"));
    }

    #[test]
    fn test_branch_name_slugify_special_chars() {
        let orch = default_orchestrator();
        let name = orch.generate_branch_name("Add OAuth2.0 support!!");
        assert!(name.starts_with("agent/add-oauth2-0-support-"));
        assert!(!name.contains('!'));
        assert!(!name.contains('.'));
    }

    #[test]
    fn test_branch_name_uniqueness() {
        let mut orch = default_orchestrator();
        // Advance the ID counter
        orch.launch_environment("task a").unwrap();
        let name1 = orch.generate_branch_name("same task");
        orch.launch_environment("task b").unwrap();
        let name2 = orch.generate_branch_name("same task");
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_branch_name_long_task_truncated() {
        let orch = default_orchestrator();
        let long_task = "a".repeat(100);
        let name = orch.generate_branch_name(&long_task);
        // "agent/" prefix + slug (<=40) + "-" + 6 hex chars
        assert!(name.len() <= 6 + 40 + 1 + 6 + 1);
    }

    #[test]
    fn test_branch_name_empty_task() {
        let orch = default_orchestrator();
        let name = orch.generate_branch_name("");
        assert!(name.starts_with("agent/"));
    }

    // --- Environment lifecycle (status transitions) ---

    #[test]
    fn test_transition_provisioning_to_cloning() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.transition_status(&id, EnvStatus::Cloning).unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Cloning);
    }

    #[test]
    fn test_transition_cloning_to_working() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.transition_status(&id, EnvStatus::Cloning).unwrap();
        orch.transition_status(&id, EnvStatus::Working).unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Working);
    }

    #[test]
    fn test_transition_working_to_committing() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.transition_status(&id, EnvStatus::Working).unwrap();
        orch.transition_status(&id, EnvStatus::Committing).unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Committing);
    }

    #[test]
    fn test_transition_to_creating_pr() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.transition_status(&id, EnvStatus::CreatingPR).unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::CreatingPR);
    }

    #[test]
    fn test_transition_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.transition_status("nonexistent", EnvStatus::Working);
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("implement feature X").unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Provisioning);

        orch.transition_status(&id, EnvStatus::Cloning).unwrap();
        orch.transition_status(&id, EnvStatus::Working).unwrap();
        orch.transition_status(&id, EnvStatus::Committing).unwrap();
        orch.transition_status(&id, EnvStatus::CreatingPR).unwrap();
        orch.transition_status(&id, EnvStatus::Completed).unwrap();

        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Completed);
    }

    // --- Docker run command ---

    #[test]
    fn test_docker_run_cmd_basic() {
        let orch = default_orchestrator();
        let id_str = "env-000001";
        let env = AgentEnvironment {
            id: id_str.to_string(),
            task_description: "test".to_string(),
            branch_name: "agent/test-abc123".to_string(),
            status: EnvStatus::Provisioning,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let cmd = orch.generate_docker_run_cmd(&env);
        assert!(cmd.starts_with("docker run --rm -d"));
        assert!(cmd.contains("--cpus=2"));
        assert!(cmd.contains("--memory=4096m"));
        assert!(cmd.contains("BRANCH=agent/test-abc123"));
        assert!(cmd.contains("vibecody/agent-env:latest"));
    }

    #[test]
    fn test_docker_run_cmd_with_resource_limits() {
        let mut config = OrchestratorConfig::default();
        config.resource_limits.cpu_cores = 4.0;
        config.resource_limits.memory_mb = 8192;
        config.resource_limits.disk_mb = 20480;
        let orch = VmOrchestrator::new(config);
        let env = AgentEnvironment {
            id: "env-test".to_string(),
            task_description: "test".to_string(),
            branch_name: "agent/test-000001".to_string(),
            status: EnvStatus::Provisioning,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let cmd = orch.generate_docker_run_cmd(&env);
        assert!(cmd.contains("--cpus=4"));
        assert!(cmd.contains("--memory=8192m"));
        assert!(cmd.contains("--storage-opt=size=20480m"));
    }

    #[test]
    fn test_docker_run_cmd_podman() {
        let mut config = OrchestratorConfig::default();
        config.runtime = RuntimeType::Podman;
        let orch = VmOrchestrator::new(config);
        let env = AgentEnvironment {
            id: "env-test".to_string(),
            task_description: "test".to_string(),
            branch_name: "agent/test-000001".to_string(),
            status: EnvStatus::Provisioning,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let cmd = orch.generate_docker_run_cmd(&env);
        assert!(cmd.starts_with("podman run"));
    }

    #[test]
    fn test_docker_run_cmd_with_git_remote() {
        let mut config = OrchestratorConfig::default();
        config.git_remote = Some("https://github.com/org/repo.git".to_string());
        let orch = VmOrchestrator::new(config);
        let env = AgentEnvironment {
            id: "env-test".to_string(),
            task_description: "test".to_string(),
            branch_name: "agent/test-000001".to_string(),
            status: EnvStatus::Provisioning,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let cmd = orch.generate_docker_run_cmd(&env);
        assert!(cmd.contains("GIT_REMOTE=https://github.com/org/repo.git"));
    }

    // --- PR spec generation ---

    #[test]
    fn test_pr_spec_basic() {
        let orch = default_orchestrator();
        let env = AgentEnvironment {
            id: "env-000001".to_string(),
            task_description: "Add user authentication".to_string(),
            branch_name: "agent/add-user-auth-000001".to_string(),
            status: EnvStatus::Completed,
            container_id: None,
            created_at: 100,
            completed_at: Some(200),
            pr_url: None,
            resource_usage: ResourceUsage { cpu_percent: 45.0, memory_mb: 1024, disk_mb: 500, duration_secs: 120 },
            log_lines: Vec::new(),
            error: None,
            files_changed: vec!["src/auth.rs".to_string(), "src/main.rs".to_string()],
        };
        let spec = orch.generate_pr_spec(&env);
        assert!(spec.title.starts_with("agent: "));
        assert!(spec.title.contains("Add user authentication"));
        assert!(spec.description.contains("src/auth.rs"));
        assert!(spec.description.contains("env-000001"));
        assert_eq!(spec.branch, "agent/add-user-auth-000001");
        assert_eq!(spec.base_branch, "main");
        assert_eq!(spec.files_changed.len(), 2);
        assert!(spec.test_plan.contains("2 files"));
    }

    #[test]
    fn test_pr_spec_long_title_truncated() {
        let orch = default_orchestrator();
        let long_desc = "a".repeat(100);
        let env = AgentEnvironment {
            id: "env-000001".to_string(),
            task_description: long_desc,
            branch_name: "agent/test-000001".to_string(),
            status: EnvStatus::Completed,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let spec = orch.generate_pr_spec(&env);
        // Title should be truncated
        assert!(spec.title.len() < 70);
    }

    #[test]
    fn test_pr_spec_no_files() {
        let orch = default_orchestrator();
        let env = AgentEnvironment {
            id: "env-000001".to_string(),
            task_description: "empty task".to_string(),
            branch_name: "agent/empty-000001".to_string(),
            status: EnvStatus::Completed,
            container_id: None,
            created_at: 0,
            completed_at: None,
            pr_url: None,
            resource_usage: ResourceUsage::default(),
            log_lines: Vec::new(),
            error: None,
            files_changed: Vec::new(),
        };
        let spec = orch.generate_pr_spec(&env);
        assert!(spec.description.contains("No files changed"));
        assert!(spec.test_plan.contains("0 files"));
    }

    // --- Conflict detection ---

    #[test]
    fn test_no_conflicts_when_no_overlap() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        orch.transition_status(&id1, EnvStatus::Working).unwrap();
        orch.transition_status(&id2, EnvStatus::Working).unwrap();

        if let Some(env) = orch.environments.get_mut(&id1) {
            env.files_changed = vec!["src/a.rs".to_string()];
        }
        if let Some(env) = orch.environments.get_mut(&id2) {
            env.files_changed = vec!["src/b.rs".to_string()];
        }

        let conflicts = orch.detect_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_detected_on_overlap() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        orch.transition_status(&id1, EnvStatus::Working).unwrap();
        orch.transition_status(&id2, EnvStatus::Working).unwrap();

        if let Some(env) = orch.environments.get_mut(&id1) {
            env.files_changed = vec!["src/shared.rs".to_string(), "src/a.rs".to_string()];
        }
        if let Some(env) = orch.environments.get_mut(&id2) {
            env.files_changed = vec!["src/shared.rs".to_string(), "src/b.rs".to_string()];
        }

        let conflicts = orch.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflicting_files, vec!["src/shared.rs".to_string()]);
        assert!(conflicts[0].resolution_suggestion.contains("Single file"));
    }

    #[test]
    fn test_conflict_multiple_files() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        orch.transition_status(&id1, EnvStatus::Working).unwrap();
        orch.transition_status(&id2, EnvStatus::Working).unwrap();

        if let Some(env) = orch.environments.get_mut(&id1) {
            env.files_changed = vec!["src/x.rs".to_string(), "src/y.rs".to_string()];
        }
        if let Some(env) = orch.environments.get_mut(&id2) {
            env.files_changed = vec!["src/x.rs".to_string(), "src/y.rs".to_string()];
        }

        let conflicts = orch.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflicting_files.len(), 2);
        assert!(conflicts[0].resolution_suggestion.contains("2 files overlap"));
    }

    #[test]
    fn test_no_conflicts_for_completed_envs() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        orch.transition_status(&id1, EnvStatus::Completed).unwrap();
        orch.transition_status(&id2, EnvStatus::Working).unwrap();

        if let Some(env) = orch.environments.get_mut(&id1) {
            env.files_changed = vec!["src/shared.rs".to_string()];
        }
        if let Some(env) = orch.environments.get_mut(&id2) {
            env.files_changed = vec!["src/shared.rs".to_string()];
        }

        let conflicts = orch.detect_conflicts();
        assert!(conflicts.is_empty());
    }

    // --- Resource tracking ---

    #[test]
    fn test_resource_usage_single_env() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        if let Some(env) = orch.environments.get_mut(&id) {
            env.resource_usage = ResourceUsage {
                cpu_percent: 50.0,
                memory_mb: 2048,
                disk_mb: 1000,
                duration_secs: 60,
            };
        }
        let total = orch.get_total_resource_usage();
        assert!((total.cpu_percent - 50.0).abs() < f32::EPSILON);
        assert_eq!(total.memory_mb, 2048);
        assert_eq!(total.disk_mb, 1000);
    }

    #[test]
    fn test_resource_usage_multiple_envs() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        if let Some(env) = orch.environments.get_mut(&id1) {
            env.resource_usage = ResourceUsage {
                cpu_percent: 30.0,
                memory_mb: 1024,
                disk_mb: 500,
                duration_secs: 60,
            };
        }
        if let Some(env) = orch.environments.get_mut(&id2) {
            env.resource_usage = ResourceUsage {
                cpu_percent: 40.0,
                memory_mb: 2048,
                disk_mb: 800,
                duration_secs: 120,
            };
        }
        let total = orch.get_total_resource_usage();
        assert!((total.cpu_percent - 70.0).abs() < f32::EPSILON);
        assert_eq!(total.memory_mb, 3072);
        assert_eq!(total.disk_mb, 1300);
        assert_eq!(total.duration_secs, 120); // max
    }

    #[test]
    fn test_resource_usage_excludes_completed() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        if let Some(env) = orch.environments.get_mut(&id) {
            env.resource_usage = ResourceUsage {
                cpu_percent: 80.0,
                memory_mb: 4096,
                disk_mb: 2000,
                duration_secs: 300,
            };
        }
        orch.transition_status(&id, EnvStatus::Completed).unwrap();
        let total = orch.get_total_resource_usage();
        assert!((total.cpu_percent - 0.0).abs() < f32::EPSILON);
        assert_eq!(total.memory_mb, 0);
    }

    // --- Complete / fail environment ---

    #[test]
    fn test_complete_environment() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("add auth").unwrap();
        orch.transition_status(&id, EnvStatus::Working).unwrap();
        let spec = orch
            .complete_environment(&id, vec!["src/auth.rs".to_string()])
            .unwrap();
        assert_eq!(orch.get_environment(&id).unwrap().status, EnvStatus::Completed);
        assert!(orch.get_environment(&id).unwrap().completed_at.is_some());
        assert_eq!(spec.files_changed, vec!["src/auth.rs".to_string()]);
    }

    #[test]
    fn test_complete_environment_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.complete_environment("nope", vec![]);
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    #[test]
    fn test_fail_environment() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.fail_environment(&id, "out of memory").unwrap();
        let env = orch.get_environment(&id).unwrap();
        assert_eq!(env.status, EnvStatus::Failed);
        assert_eq!(env.error.as_deref(), Some("out of memory"));
    }

    #[test]
    fn test_fail_environment_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.fail_environment("nope", "err");
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    // --- Cleanup ---

    #[test]
    fn test_cleanup_environment_success() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.cleanup_environment(&id).unwrap();
        assert!(orch.get_environment(&id).is_none());
        assert_eq!(orch.environments.len(), 0);
    }

    #[test]
    fn test_cleanup_environment_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.cleanup_environment("nonexistent");
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    // --- Logging ---

    #[test]
    fn test_add_and_get_logs() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.add_log(&id, "Starting provisioning...").unwrap();
        orch.add_log(&id, "Container created").unwrap();
        let logs = orch.get_logs(&id).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0], "Starting provisioning...");
        assert_eq!(logs[1], "Container created");
    }

    #[test]
    fn test_add_log_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.add_log("nope", "line");
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    #[test]
    fn test_get_logs_not_found() {
        let orch = default_orchestrator();
        let result = orch.get_logs("nope");
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    // --- Active count ---

    #[test]
    fn test_active_count_initial() {
        let orch = default_orchestrator();
        assert_eq!(orch.active_count(), 0);
    }

    #[test]
    fn test_active_count_with_mixed_statuses() {
        let mut orch = default_orchestrator();
        let id1 = orch.launch_environment("task a").unwrap();
        let id2 = orch.launch_environment("task b").unwrap();
        let id3 = orch.launch_environment("task c").unwrap();
        orch.transition_status(&id2, EnvStatus::Completed).unwrap();
        orch.transition_status(&id3, EnvStatus::Failed).unwrap();
        // Only id1 (Provisioning) is active
        assert_eq!(orch.active_count(), 1);
        // id1 still active
        assert_eq!(orch.get_environment(&id1).unwrap().status, EnvStatus::Provisioning);
    }

    // --- List environments ---

    #[test]
    fn test_list_environments_empty() {
        let orch = default_orchestrator();
        assert!(orch.list_environments().is_empty());
    }

    #[test]
    fn test_list_environments_ordered() {
        let mut orch = default_orchestrator();
        orch.launch_environment("first").unwrap();
        orch.launch_environment("second").unwrap();
        let list = orch.list_environments();
        assert_eq!(list.len(), 2);
        assert!(list[0].created_at <= list[1].created_at);
    }

    // --- Error display ---

    #[test]
    fn test_error_display_max_envs() {
        let err = OrchestratorError::MaxEnvironmentsReached;
        assert_eq!(format!("{err}"), "maximum parallel environments reached");
    }

    #[test]
    fn test_error_display_not_found() {
        let err = OrchestratorError::EnvironmentNotFound;
        assert_eq!(format!("{err}"), "environment not found");
    }

    #[test]
    fn test_error_display_provision_failed() {
        let err = OrchestratorError::ProvisionFailed("no docker".to_string());
        assert_eq!(format!("{err}"), "provision failed: no docker");
    }

    #[test]
    fn test_error_display_timeout() {
        let err = OrchestratorError::TimeoutExceeded(3600);
        assert_eq!(format!("{err}"), "timeout exceeded: 3600s");
    }

    #[test]
    fn test_error_display_invalid_config() {
        let err = OrchestratorError::InvalidConfig("bad value".to_string());
        assert_eq!(format!("{err}"), "invalid config: bad value");
    }

    #[test]
    fn test_error_display_clone_failed() {
        let err = OrchestratorError::CloneFailed("network error".to_string());
        assert_eq!(format!("{err}"), "clone failed: network error");
    }

    #[test]
    fn test_error_display_resource_exhausted() {
        let err = OrchestratorError::ResourceExhausted("OOM".to_string());
        assert_eq!(format!("{err}"), "resource exhausted: OOM");
    }

    #[test]
    fn test_error_display_cleanup_failed() {
        let err = OrchestratorError::CleanupFailed("permission denied".to_string());
        assert_eq!(format!("{err}"), "cleanup failed: permission denied");
    }

    // --- Multiple parallel environments ---

    #[test]
    fn test_multiple_parallel_envs() {
        let mut orch = default_orchestrator();
        let mut ids = Vec::new();
        for i in 0..8 {
            let id = orch.launch_environment(&format!("task {i}")).unwrap();
            ids.push(id);
        }
        assert_eq!(orch.active_count(), 8);
        assert_eq!(orch.environments.len(), 8);

        // Cannot launch a 9th
        let result = orch.launch_environment("task 9");
        assert_eq!(result.unwrap_err(), OrchestratorError::MaxEnvironmentsReached);

        // Complete one and launch again
        orch.transition_status(&ids[0], EnvStatus::Completed).unwrap();
        assert_eq!(orch.active_count(), 7);
        let new_id = orch.launch_environment("task 9 retry").unwrap();
        assert_eq!(orch.active_count(), 8);
        assert!(orch.get_environment(&new_id).is_some());
    }

    // --- Stop environment ---

    #[test]
    fn test_stop_environment() {
        let mut orch = default_orchestrator();
        let id = orch.launch_environment("task").unwrap();
        orch.stop_environment(&id).unwrap();
        let env = orch.get_environment(&id).unwrap();
        assert_eq!(env.status, EnvStatus::Failed);
        assert_eq!(env.error.as_deref(), Some("stopped by user"));
    }

    #[test]
    fn test_stop_environment_not_found() {
        let mut orch = default_orchestrator();
        let result = orch.stop_environment("nope");
        assert_eq!(result.unwrap_err(), OrchestratorError::EnvironmentNotFound);
    }

    // --- Enum Display ---

    #[test]
    fn test_runtime_type_display() {
        assert_eq!(format!("{}", RuntimeType::Docker), "docker");
        assert_eq!(format!("{}", RuntimeType::Podman), "podman");
        assert_eq!(format!("{}", RuntimeType::CloudVm), "cloud-vm");
    }

    #[test]
    fn test_env_status_display() {
        assert_eq!(format!("{}", EnvStatus::Provisioning), "provisioning");
        assert_eq!(format!("{}", EnvStatus::Cloning), "cloning");
        assert_eq!(format!("{}", EnvStatus::Working), "working");
        assert_eq!(format!("{}", EnvStatus::Committing), "committing");
        assert_eq!(format!("{}", EnvStatus::CreatingPR), "creating-pr");
        assert_eq!(format!("{}", EnvStatus::Completed), "completed");
        assert_eq!(format!("{}", EnvStatus::Failed), "failed");
        assert_eq!(format!("{}", EnvStatus::Cleaning), "cleaning");
    }

    #[test]
    fn test_get_environment_none() {
        let orch = default_orchestrator();
        assert!(orch.get_environment("nonexistent").is_none());
    }
}
