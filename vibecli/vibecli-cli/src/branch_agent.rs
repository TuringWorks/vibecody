#![allow(dead_code)]
//! Agent-per-branch workflow for VibeCody.
//!
//! Each task auto-creates a git branch and works in isolation. Multiple agents
//! can run in parallel on different branches without conflicts. On completion
//! an auto-PR is created with an AI-generated description.
//!
//! REPL commands: `/branch-agent create|list|status|complete|fail|cleanup`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum BranchAgentStatus {
    Creating,
    Working,
    Committing,
    PushingBranch,
    CreatingPR,
    Completed,
    Failed,
    Rebasing,
    CleanedUp,
}

impl std::fmt::Display for BranchAgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Working => write!(f, "working"),
            Self::Committing => write!(f, "committing"),
            Self::PushingBranch => write!(f, "pushing"),
            Self::CreatingPR => write!(f, "creating-pr"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Rebasing => write!(f, "rebasing"),
            Self::CleanedUp => write!(f, "cleaned-up"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrStatus {
    Open,
    Draft,
    Wip,
    Merged,
    Closed,
}

impl std::fmt::Display for PrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Draft => write!(f, "draft"),
            Self::Wip => write!(f, "wip"),
            Self::Merged => write!(f, "merged"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    ManualResolve,
    AutoRebase,
    SplitTask,
    WaitForMerge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BranchError {
    MaxAgentsReached,
    AgentNotFound,
    BranchExists,
    BranchConflict,
    CommitFailed,
    PushFailed,
    PRCreationFailed,
    RebaseFailed,
    CleanupFailed,
    InvalidTask,
}

impl std::fmt::Display for BranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxAgentsReached => write!(f, "maximum parallel agents reached"),
            Self::AgentNotFound => write!(f, "agent not found"),
            Self::BranchExists => write!(f, "branch already exists"),
            Self::BranchConflict => write!(f, "branch has conflicts"),
            Self::CommitFailed => write!(f, "commit failed"),
            Self::PushFailed => write!(f, "push failed"),
            Self::PRCreationFailed => write!(f, "PR creation failed"),
            Self::RebaseFailed => write!(f, "rebase failed"),
            Self::CleanupFailed => write!(f, "cleanup failed"),
            Self::InvalidTask => write!(f, "invalid task description"),
        }
    }
}

// === Configuration ===

#[derive(Debug, Clone)]
pub struct PrTemplate {
    pub title_format: String,
    pub include_test_plan: bool,
    pub include_files_changed: bool,
    pub include_agent_trace: bool,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
}

impl Default for PrTemplate {
    fn default() -> Self {
        Self {
            title_format: "{task_summary}".to_string(),
            include_test_plan: true,
            include_files_changed: true,
            include_agent_trace: true,
            reviewers: Vec::new(),
            labels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BranchAgentConfig {
    pub branch_prefix: String,
    pub base_branch: String,
    pub auto_pr: bool,
    pub auto_cleanup: bool,
    pub max_parallel_agents: usize,
    pub conflict_check_enabled: bool,
    pub conflict_check_interval_secs: u64,
    pub wip_pr_on_failure: bool,
    pub pr_template: PrTemplate,
}

impl Default for BranchAgentConfig {
    fn default() -> Self {
        Self {
            branch_prefix: "agent/".to_string(),
            base_branch: "main".to_string(),
            auto_pr: true,
            auto_cleanup: true,
            max_parallel_agents: 8,
            conflict_check_enabled: true,
            conflict_check_interval_secs: 300,
            wip_pr_on_failure: true,
            pr_template: PrTemplate::default(),
        }
    }
}

// === Core Structures ===

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub files: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u32,
    pub url: String,
    pub title: String,
    pub description: String,
    pub status: PrStatus,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct BranchAgent {
    pub id: String,
    pub task: String,
    pub branch_name: String,
    pub base_branch: String,
    pub status: BranchAgentStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub commits: Vec<CommitInfo>,
    pub files_changed: Vec<String>,
    pub pr: Option<PullRequestInfo>,
    pub error: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub agent_a_id: String,
    pub agent_b_id: String,
    pub branch_a: String,
    pub branch_b: String,
    pub conflicting_files: Vec<String>,
    pub detected_at: u64,
    pub severity: ConflictSeverity,
    pub suggestion: ConflictResolution,
}

#[derive(Debug, Clone)]
pub struct AgentSummary {
    pub total_agents: usize,
    pub active_agents: usize,
    pub completed_agents: usize,
    pub failed_agents: usize,
    pub prs_created: usize,
    pub prs_merged: usize,
    pub total_commits: usize,
    pub total_files_changed: usize,
    pub active_conflicts: usize,
}

// === Manager ===

pub struct BranchAgentManager {
    pub config: BranchAgentConfig,
    agents: HashMap<String, BranchAgent>,
    conflicts: Vec<ConflictReport>,
}

impl BranchAgentManager {
    pub fn new(config: BranchAgentConfig) -> Self {
        Self {
            config,
            agents: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Generate a short hex ID based on current time.
    fn short_id() -> String {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{:x}", millis & 0xFFFFFF)
    }

    fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Convert task text to a URL-safe slug.
    pub fn slugify(&self, text: &str) -> String {
        let slug: String = text
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        // Collapse multiple dashes and trim
        let mut result = String::with_capacity(slug.len());
        let mut prev_dash = false;
        for c in slug.chars() {
            if c == '-' {
                if !prev_dash && !result.is_empty() {
                    result.push('-');
                }
                prev_dash = true;
            } else {
                result.push(c);
                prev_dash = false;
            }
        }
        // Trim trailing dash
        while result.ends_with('-') {
            result.pop();
        }
        // Truncate to 50 chars for branch name sanity
        if result.len() > 50 {
            result.truncate(50);
            while result.ends_with('-') {
                result.pop();
            }
        }
        result
    }

    /// Generate a branch name from a task description.
    pub fn generate_branch_name(&self, task: &str) -> String {
        let slug = self.slugify(task);
        let short = Self::short_id();
        format!("{}{}-{}", self.config.branch_prefix, slug, short)
    }

    /// Create a new branch agent for a task. Returns the agent ID.
    pub fn create_agent(&mut self, task: &str) -> Result<String, BranchError> {
        let task = task.trim();
        if task.is_empty() {
            return Err(BranchError::InvalidTask);
        }

        let active = self.list_active_agents().len();
        if active >= self.config.max_parallel_agents {
            return Err(BranchError::MaxAgentsReached);
        }

        let branch_name = self.generate_branch_name(task);

        // Check for duplicate branch names among non-cleaned-up agents.
        for agent in self.agents.values() {
            if agent.branch_name == branch_name && agent.status != BranchAgentStatus::CleanedUp {
                return Err(BranchError::BranchExists);
            }
        }

        let id = format!("ba-{}", Self::short_id());
        let agent = BranchAgent {
            id: id.clone(),
            task: task.to_string(),
            branch_name,
            base_branch: self.config.base_branch.clone(),
            status: BranchAgentStatus::Creating,
            created_at: Self::now_millis(),
            completed_at: None,
            commits: Vec::new(),
            files_changed: Vec::new(),
            pr: None,
            error: None,
            trace_id: None,
        };

        self.agents.insert(id.clone(), agent);
        Ok(id)
    }

    pub fn get_agent(&self, id: &str) -> Option<&BranchAgent> {
        self.agents.get(id)
    }

    pub fn get_agent_mut(&mut self, id: &str) -> Option<&mut BranchAgent> {
        self.agents.get_mut(id)
    }

    pub fn list_agents(&self) -> Vec<&BranchAgent> {
        let mut agents: Vec<&BranchAgent> = self.agents.values().collect();
        agents.sort_by_key(|a| a.created_at);
        agents
    }

    pub fn list_active_agents(&self) -> Vec<&BranchAgent> {
        let mut agents: Vec<&BranchAgent> = self
            .agents
            .values()
            .filter(|a| matches!(
                a.status,
                BranchAgentStatus::Creating
                    | BranchAgentStatus::Working
                    | BranchAgentStatus::Committing
                    | BranchAgentStatus::PushingBranch
                    | BranchAgentStatus::CreatingPR
                    | BranchAgentStatus::Rebasing
            ))
            .collect();
        agents.sort_by_key(|a| a.created_at);
        agents
    }

    /// Record a commit for an agent.
    pub fn record_commit(
        &mut self,
        agent_id: &str,
        commit: CommitInfo,
    ) -> Result<(), BranchError> {
        let agent = self.agents.get_mut(agent_id).ok_or(BranchError::AgentNotFound)?;
        // Track files changed
        for f in &commit.files {
            if !agent.files_changed.contains(f) {
                agent.files_changed.push(f.clone());
            }
        }
        agent.commits.push(commit);
        Ok(())
    }

    /// Generate a PR title from the agent's task.
    pub fn generate_pr_title(&self, agent: &BranchAgent) -> String {
        let fmt = &self.config.pr_template.title_format;
        fmt.replace("{task_summary}", &agent.task)
    }

    /// Generate a test plan section for the PR.
    pub fn generate_test_plan(&self, agent: &BranchAgent) -> String {
        let mut plan = String::from("## Test Plan\n\n");
        if agent.files_changed.is_empty() {
            plan.push_str("- [ ] No files changed — verify intent\n");
        } else {
            plan.push_str("- [ ] Verify all changed files compile successfully\n");
            plan.push_str("- [ ] Run existing test suite\n");
            for f in &agent.files_changed {
                plan.push_str(&format!("- [ ] Review changes in `{}`\n", f));
            }
        }
        plan
    }

    /// Generate a full PR description.
    pub fn generate_pr_description(&self, agent: &BranchAgent) -> String {
        let mut desc = String::with_capacity(512);

        desc.push_str("## Summary\n\n");
        desc.push_str(&format!("Automated changes for: **{}**\n\n", agent.task));
        desc.push_str(&format!(
            "Branch: `{}` -> `{}`\n\n",
            agent.branch_name, agent.base_branch
        ));

        if self.config.pr_template.include_files_changed && !agent.files_changed.is_empty() {
            desc.push_str("## Files Changed\n\n");
            for f in &agent.files_changed {
                desc.push_str(&format!("- `{}`\n", f));
            }
            desc.push('\n');
        }

        if !agent.commits.is_empty() {
            desc.push_str("## Commits\n\n");
            for c in &agent.commits {
                desc.push_str(&format!("- `{}` {}\n", &c.hash[..7.min(c.hash.len())], c.message));
            }
            desc.push('\n');
        }

        if self.config.pr_template.include_test_plan {
            desc.push_str(&self.generate_test_plan(agent));
            desc.push('\n');
        }

        if self.config.pr_template.include_agent_trace {
            if let Some(trace) = &agent.trace_id {
                desc.push_str(&format!("## Agent Trace\n\nTrace ID: `{}`\n\n", trace));
            }
        }

        desc.push_str("---\n_Generated by VibeCody branch agent_\n");
        desc
    }

    /// Build a PullRequestInfo for a completed agent.
    fn build_pr_info(&self, agent: &BranchAgent, status: PrStatus) -> PullRequestInfo {
        let title = self.generate_pr_title(agent);
        let description = self.generate_pr_description(agent);
        let total_files: usize = agent.files_changed.len();
        // Estimate additions/deletions from commits
        let additions = agent.commits.iter().map(|c| c.files.len() * 10).sum();
        let deletions = agent.commits.iter().map(|c| c.files.len() * 3).sum();

        PullRequestInfo {
            number: (agent.created_at % 10000) as u32,
            url: format!(
                "https://github.com/repo/pull/{}",
                agent.created_at % 10000
            ),
            title,
            description,
            status,
            reviewers: self.config.pr_template.reviewers.clone(),
            labels: self.config.pr_template.labels.clone(),
            files_changed: total_files,
            additions,
            deletions,
        }
    }

    /// Mark an agent as completed and generate a PR.
    pub fn complete_agent(
        &mut self,
        agent_id: &str,
    ) -> Result<PullRequestInfo, BranchError> {
        let agent = self.agents.get_mut(agent_id).ok_or(BranchError::AgentNotFound)?;
        agent.status = BranchAgentStatus::Completed;
        agent.completed_at = Some(Self::now_millis());

        let pr = self.build_pr_info(
            &self.agents[agent_id].clone(),
            PrStatus::Open,
        );
        let agent = self.agents.get_mut(agent_id).expect("agent exists");
        agent.pr = Some(pr.clone());
        Ok(pr)
    }

    /// Mark an agent as failed. Creates a WIP PR if configured.
    pub fn fail_agent(
        &mut self,
        agent_id: &str,
        error: &str,
    ) -> Result<Option<PullRequestInfo>, BranchError> {
        let agent = self.agents.get_mut(agent_id).ok_or(BranchError::AgentNotFound)?;
        agent.status = BranchAgentStatus::Failed;
        agent.completed_at = Some(Self::now_millis());
        agent.error = Some(error.to_string());

        if self.config.wip_pr_on_failure {
            let pr = self.build_pr_info(
                &self.agents[agent_id].clone(),
                PrStatus::Wip,
            );
            let agent = self.agents.get_mut(agent_id).expect("agent exists");
            agent.pr = Some(pr.clone());
            Ok(Some(pr))
        } else {
            Ok(None)
        }
    }

    /// Detect file-level conflicts between active branches.
    pub fn detect_conflicts(&self) -> Vec<ConflictReport> {
        let active: Vec<&BranchAgent> = self.list_active_agents();
        let mut reports = Vec::new();

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
                    let severity = self.calculate_severity(&overlapping);
                    let suggestion = self.suggest_conflict_resolution_inner(&severity, overlapping.len());
                    reports.push(ConflictReport {
                        agent_a_id: a.id.clone(),
                        agent_b_id: b.id.clone(),
                        branch_a: a.branch_name.clone(),
                        branch_b: b.branch_name.clone(),
                        conflicting_files: overlapping,
                        detected_at: Self::now_millis(),
                        severity,
                        suggestion,
                    });
                }
            }
        }

        reports
    }

    fn calculate_severity(&self, conflicting_files: &[String]) -> ConflictSeverity {
        let count = conflicting_files.len();
        if count >= 5 {
            ConflictSeverity::High
        } else if count >= 2 {
            ConflictSeverity::Medium
        } else {
            ConflictSeverity::Low
        }
    }

    fn suggest_conflict_resolution_inner(
        &self,
        severity: &ConflictSeverity,
        _file_count: usize,
    ) -> ConflictResolution {
        match severity {
            ConflictSeverity::Low => ConflictResolution::AutoRebase,
            ConflictSeverity::Medium => ConflictResolution::WaitForMerge,
            ConflictSeverity::High => ConflictResolution::SplitTask,
        }
    }

    /// Public conflict resolution suggestion from a report.
    pub fn suggest_conflict_resolution(&self, conflict: &ConflictReport) -> ConflictResolution {
        self.suggest_conflict_resolution_inner(&conflict.severity, conflict.conflicting_files.len())
    }

    /// Mark an agent as rebasing.
    pub fn rebase_agent(&mut self, agent_id: &str) -> Result<(), BranchError> {
        let agent = self.agents.get_mut(agent_id).ok_or(BranchError::AgentNotFound)?;
        match agent.status {
            BranchAgentStatus::Working
            | BranchAgentStatus::Committing
            | BranchAgentStatus::Creating => {
                agent.status = BranchAgentStatus::Rebasing;
                Ok(())
            }
            _ => Err(BranchError::RebaseFailed),
        }
    }

    /// Clean up a single agent (mark as cleaned up).
    pub fn cleanup_agent(&mut self, agent_id: &str) -> Result<(), BranchError> {
        let agent = self.agents.get_mut(agent_id).ok_or(BranchError::AgentNotFound)?;
        match agent.status {
            BranchAgentStatus::Completed | BranchAgentStatus::Failed => {
                agent.status = BranchAgentStatus::CleanedUp;
                Ok(())
            }
            _ => Err(BranchError::CleanupFailed),
        }
    }

    /// Clean up all agents whose PRs are merged. Returns cleaned branch names.
    pub fn cleanup_merged_branches(&mut self) -> Vec<String> {
        let merged_ids: Vec<String> = self
            .agents
            .values()
            .filter(|a| {
                a.pr.as_ref().map(|pr| pr.status == PrStatus::Merged).unwrap_or(false)
                    && a.status != BranchAgentStatus::CleanedUp
            })
            .map(|a| a.id.clone())
            .collect();

        let mut cleaned = Vec::new();
        for id in merged_ids {
            if let Some(agent) = self.agents.get_mut(&id) {
                cleaned.push(agent.branch_name.clone());
                agent.status = BranchAgentStatus::CleanedUp;
            }
        }
        cleaned
    }

    /// Compute a summary of all agents.
    pub fn get_summary(&self) -> AgentSummary {
        let agents: Vec<&BranchAgent> = self.agents.values().collect();
        let active = agents
            .iter()
            .filter(|a| matches!(
                a.status,
                BranchAgentStatus::Creating
                    | BranchAgentStatus::Working
                    | BranchAgentStatus::Committing
                    | BranchAgentStatus::PushingBranch
                    | BranchAgentStatus::CreatingPR
                    | BranchAgentStatus::Rebasing
            ))
            .count();
        let completed = agents
            .iter()
            .filter(|a| a.status == BranchAgentStatus::Completed)
            .count();
        let failed = agents
            .iter()
            .filter(|a| a.status == BranchAgentStatus::Failed)
            .count();
        let prs_created = agents.iter().filter(|a| a.pr.is_some()).count();
        let prs_merged = agents
            .iter()
            .filter(|a| {
                a.pr.as_ref()
                    .map(|pr| pr.status == PrStatus::Merged)
                    .unwrap_or(false)
            })
            .count();
        let total_commits: usize = agents.iter().map(|a| a.commits.len()).sum();
        let total_files: usize = agents.iter().map(|a| a.files_changed.len()).sum();
        let conflicts = self.detect_conflicts();

        AgentSummary {
            total_agents: agents.len(),
            active_agents: active,
            completed_agents: completed,
            failed_agents: failed,
            prs_created,
            prs_merged,
            total_commits,
            total_files_changed: total_files,
            active_conflicts: conflicts.len(),
        }
    }

    /// Generate the git commands that would be run for a branch agent workflow.
    pub fn generate_git_commands(&self, agent: &BranchAgent) -> Vec<String> {
        let mut cmds = Vec::new();
        cmds.push(format!("git checkout {}", agent.base_branch));
        cmds.push("git pull --rebase".to_string());
        cmds.push(format!("git checkout -b {}", agent.branch_name));

        for commit in &agent.commits {
            let files_str = commit.files.join(" ");
            cmds.push(format!("git add {}", files_str));
            cmds.push(format!("git commit -m \"{}\"", commit.message));
        }

        cmds.push(format!("git push -u origin {}", agent.branch_name));

        if self.config.auto_pr {
            cmds.push(format!(
                "gh pr create --base {} --head {} --title \"{}\"",
                agent.base_branch,
                agent.branch_name,
                self.generate_pr_title(agent),
            ));
        }

        if self.config.auto_cleanup {
            cmds.push(format!("git branch -d {}", agent.branch_name));
        }

        cmds
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_manager() -> BranchAgentManager {
        BranchAgentManager::new(BranchAgentConfig::default())
    }

    fn make_commit(hash: &str, msg: &str, files: Vec<&str>) -> CommitInfo {
        CommitInfo {
            hash: hash.to_string(),
            message: msg.to_string(),
            files: files.into_iter().map(|s| s.to_string()).collect(),
            timestamp: 1000,
        }
    }

    // --- Config / Manager creation ---

    #[test]
    fn test_default_config() {
        let cfg = BranchAgentConfig::default();
        assert_eq!(cfg.branch_prefix, "agent/");
        assert_eq!(cfg.base_branch, "main");
        assert!(cfg.auto_pr);
        assert!(cfg.auto_cleanup);
        assert_eq!(cfg.max_parallel_agents, 8);
        assert!(cfg.conflict_check_enabled);
        assert_eq!(cfg.conflict_check_interval_secs, 300);
        assert!(cfg.wip_pr_on_failure);
    }

    #[test]
    fn test_default_pr_template() {
        let tmpl = PrTemplate::default();
        assert_eq!(tmpl.title_format, "{task_summary}");
        assert!(tmpl.include_test_plan);
        assert!(tmpl.include_files_changed);
        assert!(tmpl.include_agent_trace);
        assert!(tmpl.reviewers.is_empty());
        assert!(tmpl.labels.is_empty());
    }

    #[test]
    fn test_manager_new() {
        let mgr = default_manager();
        assert!(mgr.agents.is_empty());
        assert!(mgr.conflicts.is_empty());
    }

    // --- Slugify ---

    #[test]
    fn test_slugify_simple() {
        let mgr = default_manager();
        assert_eq!(mgr.slugify("add auth middleware"), "add-auth-middleware");
    }

    #[test]
    fn test_slugify_special_chars() {
        let mgr = default_manager();
        assert_eq!(mgr.slugify("fix bug #123!"), "fix-bug-123");
    }

    #[test]
    fn test_slugify_uppercase() {
        let mgr = default_manager();
        assert_eq!(mgr.slugify("Add NEW Feature"), "add-new-feature");
    }

    #[test]
    fn test_slugify_multiple_spaces() {
        let mgr = default_manager();
        assert_eq!(mgr.slugify("fix   multiple   spaces"), "fix-multiple-spaces");
    }

    #[test]
    fn test_slugify_long_name_truncates() {
        let mgr = default_manager();
        let long_task = "a".repeat(100);
        let slug = mgr.slugify(&long_task);
        assert!(slug.len() <= 50);
    }

    #[test]
    fn test_slugify_unicode() {
        let mgr = default_manager();
        let slug = mgr.slugify("add résumé support");
        // Non-ASCII becomes dashes, collapsed
        assert!(!slug.contains(' '));
        assert!(slug.starts_with("add"));
    }

    #[test]
    fn test_slugify_trailing_special_chars() {
        let mgr = default_manager();
        assert_eq!(mgr.slugify("fix bug---"), "fix-bug");
    }

    // --- Branch name generation ---

    #[test]
    fn test_generate_branch_name_has_prefix() {
        let mgr = default_manager();
        let name = mgr.generate_branch_name("add auth");
        assert!(name.starts_with("agent/add-auth-"));
    }

    #[test]
    fn test_generate_branch_name_has_short_id() {
        let mgr = default_manager();
        let name = mgr.generate_branch_name("test task");
        // Should end with a hex short id
        let parts: Vec<&str> = name.rsplitn(2, '-').collect();
        assert!(!parts[0].is_empty());
        // Verify hex
        assert!(parts[0].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_branch_name_custom_prefix() {
        let mut cfg = BranchAgentConfig::default();
        cfg.branch_prefix = "feature/bot-".to_string();
        let mgr = BranchAgentManager::new(cfg);
        let name = mgr.generate_branch_name("add login");
        assert!(name.starts_with("feature/bot-add-login-"));
    }

    // --- Agent creation ---

    #[test]
    fn test_create_agent_success() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("add authentication").unwrap();
        assert!(id.starts_with("ba-"));
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.task, "add authentication");
        assert_eq!(agent.status, BranchAgentStatus::Creating);
        assert!(agent.branch_name.starts_with("agent/"));
        assert_eq!(agent.base_branch, "main");
        assert!(agent.commits.is_empty());
        assert!(agent.files_changed.is_empty());
        assert!(agent.pr.is_none());
        assert!(agent.error.is_none());
    }

    #[test]
    fn test_create_agent_empty_task() {
        let mut mgr = default_manager();
        let err = mgr.create_agent("").unwrap_err();
        assert_eq!(err, BranchError::InvalidTask);
    }

    #[test]
    fn test_create_agent_whitespace_task() {
        let mut mgr = default_manager();
        let err = mgr.create_agent("   ").unwrap_err();
        assert_eq!(err, BranchError::InvalidTask);
    }

    #[test]
    fn test_create_agent_max_reached() {
        let mut cfg = BranchAgentConfig::default();
        cfg.max_parallel_agents = 2;
        let mut mgr = BranchAgentManager::new(cfg);
        mgr.create_agent("task one").unwrap();
        // Tiny sleep to ensure different short_id
        std::thread::sleep(std::time::Duration::from_millis(2));
        mgr.create_agent("task two").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let err = mgr.create_agent("task three").unwrap_err();
        assert_eq!(err, BranchError::MaxAgentsReached);
    }

    // --- Agent listing ---

    #[test]
    fn test_list_agents_empty() {
        let mgr = default_manager();
        assert!(mgr.list_agents().is_empty());
    }

    #[test]
    fn test_list_agents_returns_all() {
        let mut mgr = default_manager();
        mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        mgr.create_agent("task b").unwrap();
        assert_eq!(mgr.list_agents().len(), 2);
    }

    #[test]
    fn test_list_active_agents_excludes_completed() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        mgr.create_agent("task b").unwrap();
        mgr.complete_agent(&id1).unwrap();
        assert_eq!(mgr.list_active_agents().len(), 1);
    }

    #[test]
    fn test_list_active_agents_excludes_failed() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        mgr.create_agent("task b").unwrap();
        mgr.fail_agent(&id1, "oops").unwrap();
        assert_eq!(mgr.list_active_agents().len(), 1);
    }

    // --- Commit recording ---

    #[test]
    fn test_record_commit() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("add feature").unwrap();
        let commit = make_commit("abc1234", "feat: add login", vec!["src/auth.rs"]);
        mgr.record_commit(&id, commit).unwrap();
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.commits.len(), 1);
        assert_eq!(agent.files_changed, vec!["src/auth.rs"]);
    }

    #[test]
    fn test_record_commit_deduplicates_files() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("add feature").unwrap();
        let c1 = make_commit("abc1234", "first", vec!["src/lib.rs"]);
        let c2 = make_commit("def5678", "second", vec!["src/lib.rs", "src/main.rs"]);
        mgr.record_commit(&id, c1).unwrap();
        mgr.record_commit(&id, c2).unwrap();
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.commits.len(), 2);
        assert_eq!(agent.files_changed.len(), 2); // lib.rs not duplicated
    }

    #[test]
    fn test_record_commit_agent_not_found() {
        let mut mgr = default_manager();
        let commit = make_commit("abc", "msg", vec!["f.rs"]);
        let err = mgr.record_commit("nonexistent", commit).unwrap_err();
        assert_eq!(err, BranchError::AgentNotFound);
    }

    // --- Agent completion ---

    #[test]
    fn test_complete_agent() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("implement feature").unwrap();
        let commit = make_commit("abc1234", "feat: done", vec!["src/feature.rs"]);
        mgr.record_commit(&id, commit).unwrap();
        let pr = mgr.complete_agent(&id).unwrap();
        assert_eq!(pr.status, PrStatus::Open);
        assert!(pr.title.contains("implement feature"));
        assert!(pr.description.contains("implement feature"));
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.status, BranchAgentStatus::Completed);
        assert!(agent.completed_at.is_some());
        assert!(agent.pr.is_some());
    }

    #[test]
    fn test_complete_agent_not_found() {
        let mut mgr = default_manager();
        let err = mgr.complete_agent("nope").unwrap_err();
        assert_eq!(err, BranchError::AgentNotFound);
    }

    // --- Agent failure ---

    #[test]
    fn test_fail_agent_with_wip_pr() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("broken task").unwrap();
        let pr = mgr.fail_agent(&id, "compilation error").unwrap();
        assert!(pr.is_some());
        let pr = pr.unwrap();
        assert_eq!(pr.status, PrStatus::Wip);
        let agent = mgr.get_agent(&id).unwrap();
        assert_eq!(agent.status, BranchAgentStatus::Failed);
        assert_eq!(agent.error.as_deref(), Some("compilation error"));
    }

    #[test]
    fn test_fail_agent_without_wip_pr() {
        let mut cfg = BranchAgentConfig::default();
        cfg.wip_pr_on_failure = false;
        let mut mgr = BranchAgentManager::new(cfg);
        let id = mgr.create_agent("broken task").unwrap();
        let pr = mgr.fail_agent(&id, "err").unwrap();
        assert!(pr.is_none());
    }

    #[test]
    fn test_fail_agent_not_found() {
        let mut mgr = default_manager();
        let err = mgr.fail_agent("nope", "err").unwrap_err();
        assert_eq!(err, BranchError::AgentNotFound);
    }

    // --- PR generation ---

    #[test]
    fn test_generate_pr_title_default_format() {
        let mgr = default_manager();
        let mut agent = BranchAgent {
            id: "ba-1".into(),
            task: "add login page".into(),
            branch_name: "agent/add-login-page-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: Some(2000),
            commits: vec![],
            files_changed: vec![],
            pr: None,
            error: None,
            trace_id: None,
        };
        assert_eq!(mgr.generate_pr_title(&agent), "add login page");

        agent.task = "fix auth bug".into();
        assert_eq!(mgr.generate_pr_title(&agent), "fix auth bug");
    }

    #[test]
    fn test_generate_pr_description_includes_sections() {
        let mgr = default_manager();
        let agent = BranchAgent {
            id: "ba-1".into(),
            task: "add auth".into(),
            branch_name: "agent/add-auth-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: Some(2000),
            commits: vec![make_commit("abcdef1", "feat: auth", vec!["src/auth.rs"])],
            files_changed: vec!["src/auth.rs".into()],
            pr: None,
            error: None,
            trace_id: Some("trace-123".into()),
        };
        let desc = mgr.generate_pr_description(&agent);
        assert!(desc.contains("## Summary"));
        assert!(desc.contains("add auth"));
        assert!(desc.contains("## Files Changed"));
        assert!(desc.contains("`src/auth.rs`"));
        assert!(desc.contains("## Commits"));
        assert!(desc.contains("abcdef1"));
        assert!(desc.contains("## Test Plan"));
        assert!(desc.contains("## Agent Trace"));
        assert!(desc.contains("trace-123"));
        assert!(desc.contains("VibeCody branch agent"));
    }

    #[test]
    fn test_generate_test_plan_no_files() {
        let mgr = default_manager();
        let agent = BranchAgent {
            id: "ba-1".into(),
            task: "noop".into(),
            branch_name: "agent/noop-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: None,
            commits: vec![],
            files_changed: vec![],
            pr: None,
            error: None,
            trace_id: None,
        };
        let plan = mgr.generate_test_plan(&agent);
        assert!(plan.contains("No files changed"));
    }

    #[test]
    fn test_generate_test_plan_with_files() {
        let mgr = default_manager();
        let agent = BranchAgent {
            id: "ba-1".into(),
            task: "add stuff".into(),
            branch_name: "agent/add-stuff-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: None,
            commits: vec![],
            files_changed: vec!["src/a.rs".into(), "src/b.rs".into()],
            pr: None,
            error: None,
            trace_id: None,
        };
        let plan = mgr.generate_test_plan(&agent);
        assert!(plan.contains("Review changes in `src/a.rs`"));
        assert!(plan.contains("Review changes in `src/b.rs`"));
        assert!(plan.contains("Run existing test suite"));
    }

    // --- Conflict detection ---

    #[test]
    fn test_detect_conflicts_none() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        mgr.record_commit(&id1, make_commit("a1", "c1", vec!["src/a.rs"])).unwrap();
        mgr.record_commit(&id2, make_commit("b1", "c2", vec!["src/b.rs"])).unwrap();
        let conflicts = mgr.detect_conflicts();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_single_overlap() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        mgr.record_commit(&id1, make_commit("a1", "c1", vec!["src/shared.rs"])).unwrap();
        mgr.record_commit(&id2, make_commit("b1", "c2", vec!["src/shared.rs"])).unwrap();
        let conflicts = mgr.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflicting_files, vec!["src/shared.rs"]);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Low);
    }

    #[test]
    fn test_detect_conflicts_multiple_overlap() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        let files = vec!["a.rs", "b.rs", "c.rs"];
        mgr.record_commit(&id1, make_commit("a1", "c1", files.clone())).unwrap();
        mgr.record_commit(&id2, make_commit("b1", "c2", files)).unwrap();
        let conflicts = mgr.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflicting_files.len(), 3);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Medium);
    }

    #[test]
    fn test_detect_conflicts_high_severity() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        let files: Vec<&str> = vec!["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"];
        mgr.record_commit(&id1, make_commit("a1", "c1", files.clone())).unwrap();
        mgr.record_commit(&id2, make_commit("b1", "c2", files)).unwrap();
        let conflicts = mgr.detect_conflicts();
        assert_eq!(conflicts[0].severity, ConflictSeverity::High);
    }

    // --- Conflict resolution ---

    #[test]
    fn test_suggest_resolution_low() {
        let mgr = default_manager();
        let report = ConflictReport {
            agent_a_id: "a".into(),
            agent_b_id: "b".into(),
            branch_a: "ba".into(),
            branch_b: "bb".into(),
            conflicting_files: vec!["f.rs".into()],
            detected_at: 1000,
            severity: ConflictSeverity::Low,
            suggestion: ConflictResolution::AutoRebase,
        };
        assert_eq!(mgr.suggest_conflict_resolution(&report), ConflictResolution::AutoRebase);
    }

    #[test]
    fn test_suggest_resolution_medium() {
        let mgr = default_manager();
        let report = ConflictReport {
            agent_a_id: "a".into(),
            agent_b_id: "b".into(),
            branch_a: "ba".into(),
            branch_b: "bb".into(),
            conflicting_files: vec!["a.rs".into(), "b.rs".into()],
            detected_at: 1000,
            severity: ConflictSeverity::Medium,
            suggestion: ConflictResolution::WaitForMerge,
        };
        assert_eq!(
            mgr.suggest_conflict_resolution(&report),
            ConflictResolution::WaitForMerge
        );
    }

    #[test]
    fn test_suggest_resolution_high() {
        let mgr = default_manager();
        let report = ConflictReport {
            agent_a_id: "a".into(),
            agent_b_id: "b".into(),
            branch_a: "ba".into(),
            branch_b: "bb".into(),
            conflicting_files: vec!["1".into(), "2".into(), "3".into(), "4".into(), "5".into()],
            detected_at: 1000,
            severity: ConflictSeverity::High,
            suggestion: ConflictResolution::SplitTask,
        };
        assert_eq!(
            mgr.suggest_conflict_resolution(&report),
            ConflictResolution::SplitTask
        );
    }

    // --- Rebase ---

    #[test]
    fn test_rebase_working_agent() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("task").unwrap();
        mgr.get_agent_mut(&id).unwrap().status = BranchAgentStatus::Working;
        mgr.rebase_agent(&id).unwrap();
        assert_eq!(mgr.get_agent(&id).unwrap().status, BranchAgentStatus::Rebasing);
    }

    #[test]
    fn test_rebase_completed_agent_fails() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("task").unwrap();
        mgr.complete_agent(&id).unwrap();
        let err = mgr.rebase_agent(&id).unwrap_err();
        assert_eq!(err, BranchError::RebaseFailed);
    }

    #[test]
    fn test_rebase_agent_not_found() {
        let mut mgr = default_manager();
        let err = mgr.rebase_agent("nope").unwrap_err();
        assert_eq!(err, BranchError::AgentNotFound);
    }

    // --- Cleanup ---

    #[test]
    fn test_cleanup_completed_agent() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("task").unwrap();
        mgr.complete_agent(&id).unwrap();
        mgr.cleanup_agent(&id).unwrap();
        assert_eq!(mgr.get_agent(&id).unwrap().status, BranchAgentStatus::CleanedUp);
    }

    #[test]
    fn test_cleanup_failed_agent() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("task").unwrap();
        mgr.fail_agent(&id, "err").unwrap();
        mgr.cleanup_agent(&id).unwrap();
        assert_eq!(mgr.get_agent(&id).unwrap().status, BranchAgentStatus::CleanedUp);
    }

    #[test]
    fn test_cleanup_active_agent_fails() {
        let mut mgr = default_manager();
        let id = mgr.create_agent("task").unwrap();
        let err = mgr.cleanup_agent(&id).unwrap_err();
        assert_eq!(err, BranchError::CleanupFailed);
    }

    #[test]
    fn test_cleanup_merged_branches() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        mgr.complete_agent(&id1).unwrap();
        mgr.complete_agent(&id2).unwrap();
        // Simulate merge for id1
        mgr.get_agent_mut(&id1).unwrap().pr.as_mut().unwrap().status = PrStatus::Merged;
        let cleaned = mgr.cleanup_merged_branches();
        assert_eq!(cleaned.len(), 1);
        assert_eq!(mgr.get_agent(&id1).unwrap().status, BranchAgentStatus::CleanedUp);
        assert_eq!(mgr.get_agent(&id2).unwrap().status, BranchAgentStatus::Completed);
    }

    // --- Summary ---

    #[test]
    fn test_get_summary_empty() {
        let mgr = default_manager();
        let summary = mgr.get_summary();
        assert_eq!(summary.total_agents, 0);
        assert_eq!(summary.active_agents, 0);
        assert_eq!(summary.completed_agents, 0);
        assert_eq!(summary.failed_agents, 0);
        assert_eq!(summary.prs_created, 0);
        assert_eq!(summary.active_conflicts, 0);
    }

    #[test]
    fn test_get_summary_mixed() {
        let mut mgr = default_manager();
        let id1 = mgr.create_agent("task a").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = mgr.create_agent("task b").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _id3 = mgr.create_agent("task c").unwrap();

        mgr.record_commit(&id1, make_commit("a1", "c1", vec!["f1.rs"])).unwrap();
        mgr.record_commit(&id2, make_commit("b1", "c2", vec!["f2.rs"])).unwrap();
        mgr.complete_agent(&id1).unwrap();
        mgr.fail_agent(&id2, "err").unwrap();

        let summary = mgr.get_summary();
        assert_eq!(summary.total_agents, 3);
        assert_eq!(summary.active_agents, 1); // id3 still creating
        assert_eq!(summary.completed_agents, 1);
        assert_eq!(summary.failed_agents, 1);
        assert_eq!(summary.prs_created, 2); // complete + wip
        assert_eq!(summary.total_commits, 2);
        assert_eq!(summary.total_files_changed, 2);
    }

    // --- Git command generation ---

    #[test]
    fn test_generate_git_commands() {
        let mgr = default_manager();
        let agent = BranchAgent {
            id: "ba-1".into(),
            task: "add feature".into(),
            branch_name: "agent/add-feature-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: Some(2000),
            commits: vec![make_commit("abc1234", "feat: add", vec!["src/lib.rs"])],
            files_changed: vec!["src/lib.rs".into()],
            pr: None,
            error: None,
            trace_id: None,
        };
        let cmds = mgr.generate_git_commands(&agent);
        assert!(cmds.contains(&"git checkout main".to_string()));
        assert!(cmds.contains(&"git pull --rebase".to_string()));
        assert!(cmds.contains(&"git checkout -b agent/add-feature-abc".to_string()));
        assert!(cmds.iter().any(|c| c.contains("git add")));
        assert!(cmds.iter().any(|c| c.contains("git commit")));
        assert!(cmds.iter().any(|c| c.contains("git push -u origin")));
        assert!(cmds.iter().any(|c| c.contains("gh pr create")));
        assert!(cmds.iter().any(|c| c.contains("git branch -d")));
    }

    #[test]
    fn test_generate_git_commands_no_auto_pr() {
        let mut cfg = BranchAgentConfig::default();
        cfg.auto_pr = false;
        cfg.auto_cleanup = false;
        let mgr = BranchAgentManager::new(cfg);
        let agent = BranchAgent {
            id: "ba-1".into(),
            task: "task".into(),
            branch_name: "agent/task-abc".into(),
            base_branch: "main".into(),
            status: BranchAgentStatus::Completed,
            created_at: 1000,
            completed_at: Some(2000),
            commits: vec![],
            files_changed: vec![],
            pr: None,
            error: None,
            trace_id: None,
        };
        let cmds = mgr.generate_git_commands(&agent);
        assert!(!cmds.iter().any(|c| c.contains("gh pr create")));
        assert!(!cmds.iter().any(|c| c.contains("git branch -d")));
    }

    // --- Multiple parallel agents ---

    #[test]
    fn test_multiple_parallel_agents() {
        let mut mgr = default_manager();
        let mut ids = Vec::new();
        for i in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            let id = mgr.create_agent(&format!("parallel task {}", i)).unwrap();
            ids.push(id);
        }
        assert_eq!(mgr.list_agents().len(), 5);
        assert_eq!(mgr.list_active_agents().len(), 5);

        // Complete some
        mgr.complete_agent(&ids[0]).unwrap();
        mgr.complete_agent(&ids[1]).unwrap();
        assert_eq!(mgr.list_active_agents().len(), 3);
        assert_eq!(mgr.list_agents().len(), 5);
    }

    // --- Full lifecycle ---

    #[test]
    fn test_full_lifecycle() {
        let mut mgr = default_manager();

        // Create
        let id = mgr.create_agent("implement user auth").unwrap();
        assert_eq!(mgr.get_agent(&id).unwrap().status, BranchAgentStatus::Creating);

        // Work
        mgr.get_agent_mut(&id).unwrap().status = BranchAgentStatus::Working;

        // Commit
        let c1 = make_commit("aaa1111", "feat: add auth module", vec!["src/auth.rs", "src/lib.rs"]);
        mgr.record_commit(&id, c1).unwrap();
        let c2 = make_commit("bbb2222", "test: add auth tests", vec!["tests/auth_test.rs"]);
        mgr.record_commit(&id, c2).unwrap();

        assert_eq!(mgr.get_agent(&id).unwrap().commits.len(), 2);
        assert_eq!(mgr.get_agent(&id).unwrap().files_changed.len(), 3);

        // Complete
        let pr = mgr.complete_agent(&id).unwrap();
        assert_eq!(pr.status, PrStatus::Open);
        assert_eq!(pr.files_changed, 3);
        assert!(pr.description.contains("implement user auth"));

        // Cleanup
        mgr.get_agent_mut(&id).unwrap().pr.as_mut().unwrap().status = PrStatus::Merged;
        let cleaned = mgr.cleanup_merged_branches();
        assert_eq!(cleaned.len(), 1);
        assert_eq!(mgr.get_agent(&id).unwrap().status, BranchAgentStatus::CleanedUp);
    }

    // --- Display impls ---

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", BranchAgentStatus::Creating), "creating");
        assert_eq!(format!("{}", BranchAgentStatus::Working), "working");
        assert_eq!(format!("{}", BranchAgentStatus::Completed), "completed");
        assert_eq!(format!("{}", BranchAgentStatus::Failed), "failed");
        assert_eq!(format!("{}", BranchAgentStatus::CleanedUp), "cleaned-up");
    }

    #[test]
    fn test_pr_status_display() {
        assert_eq!(format!("{}", PrStatus::Open), "open");
        assert_eq!(format!("{}", PrStatus::Wip), "wip");
        assert_eq!(format!("{}", PrStatus::Merged), "merged");
    }

    #[test]
    fn test_error_display() {
        assert_eq!(format!("{}", BranchError::MaxAgentsReached), "maximum parallel agents reached");
        assert_eq!(format!("{}", BranchError::AgentNotFound), "agent not found");
        assert_eq!(format!("{}", BranchError::InvalidTask), "invalid task description");
    }
}
