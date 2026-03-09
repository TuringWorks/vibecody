use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum GitPlatform {
    GitHub,
    GitLab,
    AzureDevOps,
    Bitbucket,
    Gitea,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrState {
    Open,
    Closed,
    Merged,
    Draft,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueState {
    Open,
    Closed,
    InProgress,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Pending,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookEvent {
    Push,
    PullRequest,
    Issue,
    Comment,
    PipelineComplete,
    TagCreated,
    BranchCreated,
    BranchDeleted,
    Release,
}

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Token(String),
    OAuth(String),
    App {
        app_id: String,
        private_key: String,
    },
    Basic {
        username: String,
        password: String,
    },
}

// === Core Structures ===

#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub platform: GitPlatform,
    pub base_url: String,
    pub auth: AuthMethod,
    pub default_branch: String,
    pub project_path: String,
    pub api_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub state: PrState,
    pub author: String,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub merge_method: Option<MergeMethod>,
    pub ci_status: Option<PipelineStatus>,
    pub url: String,
    pub diff_stats: DiffStats,
}

#[derive(Debug, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub description: String,
    pub state: IssueState,
    pub author: String,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub created_at: SystemTime,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Review {
    pub id: u64,
    pub author: String,
    pub state: ReviewState,
    pub body: String,
    pub comments: Vec<ReviewComment>,
    pub submitted_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub file_path: String,
    pub line: usize,
    pub body: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: u64,
    pub name: String,
    pub status: PipelineStatus,
    pub branch: String,
    pub commit_sha: String,
    pub stages: Vec<PipelineStage>,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub status: PipelineStatus,
    pub jobs: Vec<PipelineJob>,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PipelineJob {
    pub name: String,
    pub status: PipelineStatus,
    pub log_url: Option<String>,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_path: String,
    pub default_branch: String,
    pub description: Option<String>,
    pub visibility: String,
    pub clone_url: String,
    pub web_url: String,
    pub created_at: SystemTime,
    pub language: Option<String>,
    pub stars: u64,
    pub forks: u64,
}

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: Option<String>,
    pub active: bool,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: SystemTime,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub sha: String,
    pub is_protected: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct StatusCheck {
    pub context: String,
    pub state: PipelineStatus,
    pub description: String,
    pub target_url: Option<String>,
}

// === API URL Builder ===

pub struct ApiUrlBuilder {
    pub platform: GitPlatform,
    pub base_url: String,
    pub project_path: String,
}

impl ApiUrlBuilder {
    pub fn new(platform: GitPlatform, base_url: &str, project: &str) -> Self {
        Self {
            platform,
            base_url: base_url.trim_end_matches('/').to_string(),
            project_path: project.to_string(),
        }
    }

    /// Returns the URL for listing/creating pull requests.
    /// GitHub: /repos/{owner}/{repo}/pulls
    /// GitLab: /projects/{encoded}/merge_requests
    /// Azure:  /{org}/{project}/_apis/git/repositories/{repo}/pullrequests
    pub fn pull_requests_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}/pulls", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!("{}/projects/{}/merge_requests", self.base_url, encoded)
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}}/{}/{}/_apis/git/repositories/{}/pullrequests",
                    self.base_url, org, project, repo
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/pullrequests",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}/pulls", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/pulls", self.base_url, self.project_path)
            }
        }
    }

    pub fn issues_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}/issues", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!("{}/projects/{}/issues", self.base_url, encoded)
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project) = if parts.len() >= 2 {
                    (parts[0], parts[1])
                } else {
                    (self.project_path.as_str(), self.project_path.as_str())
                };
                format!(
                    "{}}/{}/{}/_apis/wit/workitems",
                    self.base_url, org, project
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/issues",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}/issues", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/issues", self.base_url, self.project_path)
            }
        }
    }

    /// GitHub: /repos/.../actions/runs
    /// GitLab: /projects/.../pipelines
    /// Azure:  /{org}/{project}/_apis/pipelines
    pub fn pipelines_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!(
                    "{}/repos/{}/actions/runs",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!("{}/projects/{}/pipelines", self.base_url, encoded)
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project) = if parts.len() >= 2 {
                    (parts[0], parts[1])
                } else {
                    (self.project_path.as_str(), self.project_path.as_str())
                };
                format!("{}}/{}/{}/_apis/pipelines", self.base_url, org, project)
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/pipelines",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!(
                    "{}/repos/{}/actions/runs",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/pipelines", self.base_url, self.project_path)
            }
        }
    }

    pub fn branches_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}/branches", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!(
                    "{}/projects/{}/repository/branches",
                    self.base_url, encoded
                )
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}}/{}/{}/_apis/git/repositories/{}/refs",
                    self.base_url, org, project, repo
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/refs/branches",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}/branches", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/branches", self.base_url, self.project_path)
            }
        }
    }

    pub fn commits_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}/commits", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!(
                    "{}/projects/{}/repository/commits",
                    self.base_url, encoded
                )
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}/{}/{}/_apis/git/repositories/{}/commits",
                    self.base_url, org, project, repo
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/commits",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}/commits", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/commits", self.base_url, self.project_path)
            }
        }
    }

    pub fn reviews_url(&self, pr_number: u64) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!(
                    "{}/repos/{}/pulls/{}/reviews",
                    self.base_url, self.project_path, pr_number
                )
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!(
                    "{}/projects/{}/merge_requests/{}/approvals",
                    self.base_url, encoded, pr_number
                )
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}/{}/{}/_apis/git/repositories/{}/pullrequests/{}/reviewers",
                    self.base_url, org, project, repo, pr_number
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/pullrequests/{}/activity",
                    self.base_url, self.project_path, pr_number
                )
            }
            GitPlatform::Gitea => {
                format!(
                    "{}/repos/{}/pulls/{}/reviews",
                    self.base_url, self.project_path, pr_number
                )
            }
            GitPlatform::Custom(_) => {
                format!(
                    "{}/repos/{}/pulls/{}/reviews",
                    self.base_url, self.project_path, pr_number
                )
            }
        }
    }

    pub fn status_checks_url(&self, sha: &str) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!(
                    "{}/repos/{}/commits/{}/status",
                    self.base_url, self.project_path, sha
                )
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!(
                    "{}/projects/{}/repository/commits/{}/statuses",
                    self.base_url, encoded, sha
                )
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}/{}/{}/_apis/git/repositories/{}/commits/{}/statuses",
                    self.base_url, org, project, repo, sha
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/commit/{}/statuses",
                    self.base_url, self.project_path, sha
                )
            }
            GitPlatform::Gitea => {
                format!(
                    "{}/repos/{}/commits/{}/status",
                    self.base_url, self.project_path, sha
                )
            }
            GitPlatform::Custom(_) => {
                format!(
                    "{}/repos/{}/commits/{}/status",
                    self.base_url, self.project_path, sha
                )
            }
        }
    }

    pub fn webhooks_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}/hooks", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!("{}/projects/{}/hooks", self.base_url, encoded)
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let org = if !parts.is_empty() { parts[0] } else { &self.project_path };
                format!("{}/_apis/hooks/subscriptions", self.base_url)
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/hooks",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}/hooks", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}/hooks", self.base_url, self.project_path)
            }
        }
    }

    pub fn repository_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!("{}/repos/{}", self.base_url, self.project_path)
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!("{}/projects/{}", self.base_url, encoded)
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}/{}/{}/_apis/git/repositories/{}",
                    self.base_url, org, project, repo
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}",
                    self.base_url, self.project_path
                )
            }
            GitPlatform::Gitea => {
                format!("{}/repos/{}", self.base_url, self.project_path)
            }
            GitPlatform::Custom(_) => {
                format!("{}/repos/{}", self.base_url, self.project_path)
            }
        }
    }

    pub fn compare_url(&self, base: &str, head: &str) -> String {
        match &self.platform {
            GitPlatform::GitHub => {
                format!(
                    "{}/repos/{}/compare/{}...{}",
                    self.base_url, self.project_path, base, head
                )
            }
            GitPlatform::GitLab => {
                let encoded = self.project_path.replace('/', "%2F");
                format!(
                    "{}/projects/{}/repository/compare?from={}&to={}",
                    self.base_url, encoded, base, head
                )
            }
            GitPlatform::AzureDevOps => {
                let parts: Vec<&str> = self.project_path.splitn(3, '/').collect();
                let (org, project, repo) = match parts.len() {
                    3 => (parts[0], parts[1], parts[2]),
                    2 => (parts[0], parts[1], parts[1]),
                    _ => (self.project_path.as_str(), self.project_path.as_str(), self.project_path.as_str()),
                };
                format!(
                    "{}/{}/{}/_apis/git/repositories/{}/diffs/commits?baseVersion={}&targetVersion={}",
                    self.base_url, org, project, repo, base, head
                )
            }
            GitPlatform::Bitbucket => {
                format!(
                    "{}/repositories/{}/diff/{}..{}",
                    self.base_url, self.project_path, base, head
                )
            }
            GitPlatform::Gitea => {
                format!(
                    "{}/repos/{}/compare/{}...{}",
                    self.base_url, self.project_path, base, head
                )
            }
            GitPlatform::Custom(_) => {
                format!(
                    "{}/repos/{}/compare/{}...{}",
                    self.base_url, self.project_path, base, head
                )
            }
        }
    }
}

// === PlatformConfig factory methods ===

impl PlatformConfig {
    pub fn github(token: &str, repo: &str) -> Self {
        Self {
            platform: GitPlatform::GitHub,
            base_url: "https://api.github.com".to_string(),
            auth: AuthMethod::Token(token.to_string()),
            default_branch: "main".to_string(),
            project_path: repo.to_string(),
            api_version: Some("2022-11-28".to_string()),
        }
    }

    pub fn gitlab(token: &str, base_url: &str, project: &str) -> Self {
        Self {
            platform: GitPlatform::GitLab,
            base_url: format!("{}/api/v4", base_url.trim_end_matches('/')),
            auth: AuthMethod::Token(token.to_string()),
            default_branch: "main".to_string(),
            project_path: project.to_string(),
            api_version: Some("v4".to_string()),
        }
    }

    pub fn azure_devops(token: &str, org: &str, project: &str, repo: &str) -> Self {
        Self {
            platform: GitPlatform::AzureDevOps,
            base_url: "https://dev.azure.com".to_string(),
            auth: AuthMethod::Token(token.to_string()),
            default_branch: "main".to_string(),
            project_path: format!("{}/{}/{}", org, project, repo),
            api_version: Some("7.0".to_string()),
        }
    }

    pub fn bitbucket(token: &str, workspace: &str, repo: &str) -> Self {
        Self {
            platform: GitPlatform::Bitbucket,
            base_url: "https://api.bitbucket.org/2.0".to_string(),
            auth: AuthMethod::Token(token.to_string()),
            default_branch: "main".to_string(),
            project_path: format!("{}/{}", workspace, repo),
            api_version: Some("2.0".to_string()),
        }
    }

    pub fn gitea(token: &str, base_url: &str, repo: &str) -> Self {
        Self {
            platform: GitPlatform::Gitea,
            base_url: format!("{}/api/v1", base_url.trim_end_matches('/')),
            auth: AuthMethod::Token(token.to_string()),
            default_branch: "main".to_string(),
            project_path: repo.to_string(),
            api_version: Some("v1".to_string()),
        }
    }

    pub fn api_base_url(&self) -> String {
        match &self.platform {
            GitPlatform::GitHub => "https://api.github.com".to_string(),
            GitPlatform::GitLab => self.base_url.clone(),
            GitPlatform::AzureDevOps => "https://dev.azure.com".to_string(),
            GitPlatform::Bitbucket => "https://api.bitbucket.org/2.0".to_string(),
            GitPlatform::Gitea => self.base_url.clone(),
            GitPlatform::Custom(url) => url.clone(),
        }
    }
}

// === GitPlatformClient ===

pub struct GitPlatformClient {
    pub config: PlatformConfig,
    pub pull_requests: Vec<PullRequest>,
    pub issues: Vec<Issue>,
    pub pipelines: Vec<Pipeline>,
    pub branches: Vec<BranchInfo>,
    pub webhooks: Vec<WebhookConfig>,
    pub repositories: Vec<Repository>,
    next_pr_id: u64,
    next_issue_id: u64,
    next_webhook_id: u64,
}

impl GitPlatformClient {
    pub fn new(config: PlatformConfig) -> Self {
        Self {
            config,
            pull_requests: Vec::new(),
            issues: Vec::new(),
            pipelines: Vec::new(),
            branches: Vec::new(),
            webhooks: Vec::new(),
            repositories: Vec::new(),
            next_pr_id: 1,
            next_issue_id: 1,
            next_webhook_id: 1,
        }
    }

    pub fn platform_name(&self) -> &str {
        match &self.config.platform {
            GitPlatform::GitHub => "GitHub",
            GitPlatform::GitLab => "GitLab",
            GitPlatform::AzureDevOps => "Azure DevOps",
            GitPlatform::Bitbucket => "Bitbucket",
            GitPlatform::Gitea => "Gitea",
            GitPlatform::Custom(name) => name.as_str(),
        }
    }

    // --- PR operations ---

    pub fn create_pull_request(
        &mut self,
        title: &str,
        description: &str,
        source: &str,
        target: &str,
    ) -> PullRequest {
        let id = self.next_pr_id;
        self.next_pr_id += 1;
        let now = SystemTime::now();

        let pr_term = match &self.config.platform {
            GitPlatform::GitLab => "merge_requests",
            _ => "pull",
        };

        let url = format!(
            "{}/{}/{}/{}",
            self.config.base_url, self.config.project_path, pr_term, id
        );

        let pr = PullRequest {
            id,
            number: id,
            title: title.to_string(),
            description: description.to_string(),
            source_branch: source.to_string(),
            target_branch: target.to_string(),
            state: PrState::Open,
            author: "current-user".to_string(),
            reviewers: Vec::new(),
            labels: Vec::new(),
            created_at: now,
            updated_at: now,
            merge_method: None,
            ci_status: None,
            url,
            diff_stats: DiffStats {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
        };
        self.pull_requests.push(pr.clone());
        pr
    }

    pub fn get_pull_request(&self, number: u64) -> Option<&PullRequest> {
        self.pull_requests.iter().find(|pr| pr.number == number)
    }

    pub fn list_pull_requests(&self, state: Option<&PrState>) -> Vec<&PullRequest> {
        match state {
            Some(s) => self.pull_requests.iter().filter(|pr| &pr.state == s).collect(),
            None => self.pull_requests.iter().collect(),
        }
    }

    pub fn update_pull_request(
        &mut self,
        number: u64,
        title: Option<&str>,
        description: Option<&str>,
    ) -> bool {
        if let Some(pr) = self.pull_requests.iter_mut().find(|pr| pr.number == number) {
            if let Some(t) = title {
                pr.title = t.to_string();
            }
            if let Some(d) = description {
                pr.description = d.to_string();
            }
            pr.updated_at = SystemTime::now();
            true
        } else {
            false
        }
    }

    pub fn close_pull_request(&mut self, number: u64) -> bool {
        if let Some(pr) = self.pull_requests.iter_mut().find(|pr| pr.number == number) {
            pr.state = PrState::Closed;
            pr.updated_at = SystemTime::now();
            true
        } else {
            false
        }
    }

    pub fn merge_pull_request(&mut self, number: u64, method: MergeMethod) -> bool {
        if let Some(pr) = self.pull_requests.iter_mut().find(|pr| pr.number == number) {
            if pr.state != PrState::Open && pr.state != PrState::Draft {
                return false;
            }
            pr.state = PrState::Merged;
            pr.merge_method = Some(method);
            pr.updated_at = SystemTime::now();
            true
        } else {
            false
        }
    }

    pub fn add_reviewer(&mut self, pr_number: u64, reviewer: &str) -> bool {
        if let Some(pr) = self.pull_requests.iter_mut().find(|pr| pr.number == pr_number) {
            if !pr.reviewers.contains(&reviewer.to_string()) {
                pr.reviewers.push(reviewer.to_string());
            }
            pr.updated_at = SystemTime::now();
            true
        } else {
            false
        }
    }

    pub fn add_label(&mut self, pr_number: u64, label: &str) -> bool {
        if let Some(pr) = self.pull_requests.iter_mut().find(|pr| pr.number == pr_number) {
            if !pr.labels.contains(&label.to_string()) {
                pr.labels.push(label.to_string());
            }
            pr.updated_at = SystemTime::now();
            true
        } else {
            false
        }
    }

    // --- Issue operations ---

    pub fn create_issue(&mut self, title: &str, description: &str) -> Issue {
        let id = self.next_issue_id;
        self.next_issue_id += 1;
        let now = SystemTime::now();

        let url = format!(
            "{}/{}/issues/{}",
            self.config.base_url, self.config.project_path, id
        );

        let issue = Issue {
            id,
            number: id,
            title: title.to_string(),
            description: description.to_string(),
            state: IssueState::Open,
            author: "current-user".to_string(),
            assignees: Vec::new(),
            labels: Vec::new(),
            milestone: None,
            created_at: now,
            url,
        };
        self.issues.push(issue.clone());
        issue
    }

    pub fn get_issue(&self, number: u64) -> Option<&Issue> {
        self.issues.iter().find(|i| i.number == number)
    }

    pub fn list_issues(&self, state: Option<&IssueState>) -> Vec<&Issue> {
        match state {
            Some(s) => self.issues.iter().filter(|i| &i.state == s).collect(),
            None => self.issues.iter().collect(),
        }
    }

    pub fn close_issue(&mut self, number: u64) -> bool {
        if let Some(issue) = self.issues.iter_mut().find(|i| i.number == number) {
            issue.state = IssueState::Closed;
            true
        } else {
            false
        }
    }

    pub fn assign_issue(&mut self, number: u64, assignee: &str) -> bool {
        if let Some(issue) = self.issues.iter_mut().find(|i| i.number == number) {
            if !issue.assignees.contains(&assignee.to_string()) {
                issue.assignees.push(assignee.to_string());
            }
            true
        } else {
            false
        }
    }

    // --- Pipeline/CI operations ---

    pub fn get_pipeline(&self, id: u64) -> Option<&Pipeline> {
        self.pipelines.iter().find(|p| p.id == id)
    }

    pub fn list_pipelines(&self) -> Vec<&Pipeline> {
        self.pipelines.iter().collect()
    }

    pub fn add_pipeline(&mut self, pipeline: Pipeline) {
        self.pipelines.push(pipeline);
    }

    pub fn latest_pipeline_for_branch(&self, branch: &str) -> Option<&Pipeline> {
        self.pipelines
            .iter()
            .filter(|p| p.branch == branch)
            .last()
    }

    // --- Branch operations ---

    pub fn list_branches(&self) -> Vec<&BranchInfo> {
        self.branches.iter().collect()
    }

    pub fn create_branch(&mut self, name: &str, sha: &str) -> BranchInfo {
        let branch = BranchInfo {
            name: name.to_string(),
            sha: sha.to_string(),
            is_protected: false,
            is_default: false,
        };
        self.branches.push(branch.clone());
        branch
    }

    pub fn delete_branch(&mut self, name: &str) -> bool {
        let before = self.branches.len();
        self.branches.retain(|b| b.name != name);
        self.branches.len() < before
    }

    pub fn get_default_branch(&self) -> Option<&BranchInfo> {
        self.branches.iter().find(|b| b.is_default)
    }

    // --- Webhook operations ---

    pub fn create_webhook(&mut self, url: &str, events: Vec<WebhookEvent>) -> WebhookConfig {
        let id = format!("wh-{}", self.next_webhook_id);
        self.next_webhook_id += 1;

        let wh = WebhookConfig {
            id,
            url: url.to_string(),
            events,
            secret: None,
            active: true,
            created_at: SystemTime::now(),
        };
        self.webhooks.push(wh.clone());
        wh
    }

    pub fn list_webhooks(&self) -> Vec<&WebhookConfig> {
        self.webhooks.iter().collect()
    }

    pub fn delete_webhook(&mut self, id: &str) -> bool {
        let before = self.webhooks.len();
        self.webhooks.retain(|w| w.id != id);
        self.webhooks.len() < before
    }

    // --- Status checks ---

    pub fn create_status_check(
        &self,
        sha: &str,
        context: &str,
        state: PipelineStatus,
        description: &str,
    ) -> StatusCheck {
        let target_url = Some(format!(
            "{}/{}/commits/{}",
            self.config.base_url, self.config.project_path, sha
        ));
        StatusCheck {
            context: context.to_string(),
            state,
            description: description.to_string(),
            target_url,
        }
    }

    pub fn api_urls(&self) -> ApiUrlBuilder {
        ApiUrlBuilder::new(
            self.config.platform.clone(),
            &self.config.base_url,
            &self.config.project_path,
        )
    }

    // --- Statistics ---

    pub fn open_pr_count(&self) -> usize {
        self.pull_requests
            .iter()
            .filter(|pr| pr.state == PrState::Open)
            .count()
    }

    pub fn open_issue_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.state == IssueState::Open)
            .count()
    }

    pub fn recent_commits(&self, _branch: &str) -> Vec<&CommitInfo> {
        Vec::new()
    }
}

// === Multi-Platform Manager ===

pub struct PlatformManager {
    pub clients: HashMap<String, GitPlatformClient>,
    pub default_platform: Option<String>,
}

impl PlatformManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            default_platform: None,
        }
    }

    pub fn add_platform(&mut self, name: &str, config: PlatformConfig) -> &mut GitPlatformClient {
        self.clients
            .insert(name.to_string(), GitPlatformClient::new(config));
        if self.default_platform.is_none() {
            self.default_platform = Some(name.to_string());
        }
        self.clients.get_mut(name).expect("just inserted")
    }

    pub fn get_platform(&self, name: &str) -> Option<&GitPlatformClient> {
        self.clients.get(name)
    }

    pub fn get_platform_mut(&mut self, name: &str) -> Option<&mut GitPlatformClient> {
        self.clients.get_mut(name)
    }

    pub fn set_default(&mut self, name: &str) -> bool {
        if self.clients.contains_key(name) {
            self.default_platform = Some(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn default_client(&self) -> Option<&GitPlatformClient> {
        self.default_platform
            .as_ref()
            .and_then(|name| self.clients.get(name))
    }

    pub fn default_client_mut(&mut self) -> Option<&mut GitPlatformClient> {
        if let Some(name) = self.default_platform.clone() {
            self.clients.get_mut(&name)
        } else {
            None
        }
    }

    pub fn list_platforms(&self) -> Vec<(&str, &str)> {
        self.clients
            .iter()
            .map(|(name, client)| (name.as_str(), client.platform_name()))
            .collect()
    }

    pub fn remove_platform(&mut self, name: &str) -> bool {
        let removed = self.clients.remove(name).is_some();
        if removed {
            if self.default_platform.as_deref() == Some(name) {
                self.default_platform = self.clients.keys().next().cloned();
            }
        }
        removed
    }

    /// Syncs a PR description across target platforms by returning which platforms
    /// were found and could receive the sync. In a real implementation this would
    /// call each platform's API; here we validate the targets exist.
    pub fn sync_pr_across_platforms(
        &self,
        pr: &PullRequest,
        target_platforms: &[&str],
    ) -> Vec<String> {
        let mut synced = Vec::new();
        for &target in target_platforms {
            if self.clients.contains_key(target) {
                synced.push(target.to_string());
            }
        }
        synced
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn github_config() -> PlatformConfig {
        PlatformConfig::github("ghp_test_token", "octocat/hello-world")
    }

    fn gitlab_config() -> PlatformConfig {
        PlatformConfig::gitlab("glpat-test", "https://gitlab.example.com", "group/project")
    }

    fn azure_config() -> PlatformConfig {
        PlatformConfig::azure_devops("pat-test", "myorg", "myproject", "myrepo")
    }

    fn bitbucket_config() -> PlatformConfig {
        PlatformConfig::bitbucket("bb-token", "myworkspace", "myrepo")
    }

    fn gitea_config() -> PlatformConfig {
        PlatformConfig::gitea("gt-token", "https://gitea.local", "user/repo")
    }

    // --- Config factory tests ---

    #[test]
    fn test_github_config_factory() {
        let cfg = github_config();
        assert_eq!(cfg.platform, GitPlatform::GitHub);
        assert_eq!(cfg.base_url, "https://api.github.com");
        assert_eq!(cfg.project_path, "octocat/hello-world");
        assert_eq!(cfg.default_branch, "main");
        assert!(cfg.api_version.is_some());
    }

    #[test]
    fn test_gitlab_config_factory() {
        let cfg = gitlab_config();
        assert_eq!(cfg.platform, GitPlatform::GitLab);
        assert_eq!(cfg.base_url, "https://gitlab.example.com/api/v4");
        assert_eq!(cfg.project_path, "group/project");
    }

    #[test]
    fn test_azure_config_factory() {
        let cfg = azure_config();
        assert_eq!(cfg.platform, GitPlatform::AzureDevOps);
        assert_eq!(cfg.base_url, "https://dev.azure.com");
        assert_eq!(cfg.project_path, "myorg/myproject/myrepo");
    }

    #[test]
    fn test_bitbucket_config_factory() {
        let cfg = bitbucket_config();
        assert_eq!(cfg.platform, GitPlatform::Bitbucket);
        assert_eq!(cfg.base_url, "https://api.bitbucket.org/2.0");
        assert_eq!(cfg.project_path, "myworkspace/myrepo");
    }

    #[test]
    fn test_gitea_config_factory() {
        let cfg = gitea_config();
        assert_eq!(cfg.platform, GitPlatform::Gitea);
        assert_eq!(cfg.base_url, "https://gitea.local/api/v1");
        assert_eq!(cfg.project_path, "user/repo");
    }

    #[test]
    fn test_api_base_url_github() {
        let cfg = github_config();
        assert_eq!(cfg.api_base_url(), "https://api.github.com");
    }

    #[test]
    fn test_api_base_url_azure() {
        let cfg = azure_config();
        assert_eq!(cfg.api_base_url(), "https://dev.azure.com");
    }

    #[test]
    fn test_api_base_url_bitbucket() {
        let cfg = bitbucket_config();
        assert_eq!(cfg.api_base_url(), "https://api.bitbucket.org/2.0");
    }

    #[test]
    fn test_api_base_url_custom() {
        let cfg = PlatformConfig {
            platform: GitPlatform::Custom("https://custom.git".to_string()),
            base_url: "https://custom.git/api".to_string(),
            auth: AuthMethod::Token("t".to_string()),
            default_branch: "main".to_string(),
            project_path: "u/r".to_string(),
            api_version: None,
        };
        assert_eq!(cfg.api_base_url(), "https://custom.git");
    }

    // --- URL builder tests: GitHub ---

    #[test]
    fn test_github_pr_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "octocat/repo");
        assert_eq!(b.pull_requests_url(), "https://api.github.com/repos/octocat/repo/pulls");
    }

    #[test]
    fn test_github_issues_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "octocat/repo");
        assert_eq!(b.issues_url(), "https://api.github.com/repos/octocat/repo/issues");
    }

    #[test]
    fn test_github_pipelines_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "octocat/repo");
        assert_eq!(b.pipelines_url(), "https://api.github.com/repos/octocat/repo/actions/runs");
    }

    #[test]
    fn test_github_branches_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.branches_url(), "https://api.github.com/repos/o/r/branches");
    }

    #[test]
    fn test_github_commits_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.commits_url(), "https://api.github.com/repos/o/r/commits");
    }

    #[test]
    fn test_github_reviews_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.reviews_url(42), "https://api.github.com/repos/o/r/pulls/42/reviews");
    }

    #[test]
    fn test_github_status_checks_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.status_checks_url("abc123"), "https://api.github.com/repos/o/r/commits/abc123/status");
    }

    #[test]
    fn test_github_webhooks_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.webhooks_url(), "https://api.github.com/repos/o/r/hooks");
    }

    #[test]
    fn test_github_repository_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.repository_url(), "https://api.github.com/repos/o/r");
    }

    #[test]
    fn test_github_compare_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com", "o/r");
        assert_eq!(b.compare_url("main", "feature"), "https://api.github.com/repos/o/r/compare/main...feature");
    }

    // --- URL builder tests: GitLab ---

    #[test]
    fn test_gitlab_pr_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "group/project");
        assert_eq!(b.pull_requests_url(), "https://gl.com/api/v4/projects/group%2Fproject/merge_requests");
    }

    #[test]
    fn test_gitlab_issues_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.issues_url(), "https://gl.com/api/v4/projects/g%2Fp/issues");
    }

    #[test]
    fn test_gitlab_pipelines_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.pipelines_url(), "https://gl.com/api/v4/projects/g%2Fp/pipelines");
    }

    #[test]
    fn test_gitlab_branches_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.branches_url(), "https://gl.com/api/v4/projects/g%2Fp/repository/branches");
    }

    #[test]
    fn test_gitlab_commits_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.commits_url(), "https://gl.com/api/v4/projects/g%2Fp/repository/commits");
    }

    #[test]
    fn test_gitlab_reviews_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.reviews_url(10), "https://gl.com/api/v4/projects/g%2Fp/merge_requests/10/approvals");
    }

    #[test]
    fn test_gitlab_compare_url() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        assert_eq!(b.compare_url("main", "dev"), "https://gl.com/api/v4/projects/g%2Fp/repository/compare?from=main&to=dev");
    }

    #[test]
    fn test_gitlab_nested_project_encoding() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "group/sub/project");
        assert!(b.pull_requests_url().contains("group%2Fsub%2Fproject"));
    }

    // --- URL builder tests: Azure DevOps ---

    #[test]
    fn test_azure_pr_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert_eq!(b.pull_requests_url(), "https://dev.azure.com/org/proj/_apis/git/repositories/repo/pullrequests");
    }

    #[test]
    fn test_azure_pipelines_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert_eq!(b.pipelines_url(), "https://dev.azure.com/org/proj/_apis/pipelines");
    }

    #[test]
    fn test_azure_branches_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert_eq!(b.branches_url(), "https://dev.azure.com/org/proj/_apis/git/repositories/repo/refs");
    }

    #[test]
    fn test_azure_compare_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert!(b.compare_url("main", "dev").contains("baseVersion=main"));
        assert!(b.compare_url("main", "dev").contains("targetVersion=dev"));
    }

    #[test]
    fn test_azure_issues_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert_eq!(b.issues_url(), "https://dev.azure.com/org/proj/_apis/wit/workitems");
    }

    #[test]
    fn test_azure_webhooks_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        assert_eq!(b.webhooks_url(), "https://dev.azure.com/_apis/hooks/subscriptions");
    }

    // --- URL builder tests: Bitbucket ---

    #[test]
    fn test_bitbucket_pr_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Bitbucket, "https://api.bitbucket.org/2.0", "ws/repo");
        assert_eq!(b.pull_requests_url(), "https://api.bitbucket.org/2.0/repositories/ws/repo/pullrequests");
    }

    #[test]
    fn test_bitbucket_branches_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Bitbucket, "https://api.bitbucket.org/2.0", "ws/repo");
        assert_eq!(b.branches_url(), "https://api.bitbucket.org/2.0/repositories/ws/repo/refs/branches");
    }

    #[test]
    fn test_bitbucket_compare_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Bitbucket, "https://api.bitbucket.org/2.0", "ws/repo");
        assert_eq!(b.compare_url("main", "dev"), "https://api.bitbucket.org/2.0/repositories/ws/repo/diff/main..dev");
    }

    // --- URL builder tests: Gitea ---

    #[test]
    fn test_gitea_pr_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Gitea, "https://gitea.local/api/v1", "u/r");
        assert_eq!(b.pull_requests_url(), "https://gitea.local/api/v1/repos/u/r/pulls");
    }

    #[test]
    fn test_gitea_pipelines_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Gitea, "https://gitea.local/api/v1", "u/r");
        assert_eq!(b.pipelines_url(), "https://gitea.local/api/v1/repos/u/r/actions/runs");
    }

    // --- PR CRUD tests ---

    #[test]
    fn test_create_pull_request() {
        let mut client = GitPlatformClient::new(github_config());
        let pr = client.create_pull_request("Add feature", "Description here", "feature", "main");
        assert_eq!(pr.title, "Add feature");
        assert_eq!(pr.source_branch, "feature");
        assert_eq!(pr.target_branch, "main");
        assert_eq!(pr.state, PrState::Open);
        assert_eq!(pr.number, 1);
    }

    #[test]
    fn test_get_pull_request() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR1", "d", "f", "m");
        assert!(client.get_pull_request(1).is_some());
        assert!(client.get_pull_request(99).is_none());
    }

    #[test]
    fn test_list_pull_requests_all() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR1", "", "f1", "m");
        client.create_pull_request("PR2", "", "f2", "m");
        assert_eq!(client.list_pull_requests(None).len(), 2);
    }

    #[test]
    fn test_list_pull_requests_by_state() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR1", "", "f1", "m");
        client.create_pull_request("PR2", "", "f2", "m");
        client.close_pull_request(1);
        assert_eq!(client.list_pull_requests(Some(&PrState::Open)).len(), 1);
        assert_eq!(client.list_pull_requests(Some(&PrState::Closed)).len(), 1);
    }

    #[test]
    fn test_update_pull_request() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("Old", "old desc", "f", "m");
        assert!(client.update_pull_request(1, Some("New"), None));
        assert_eq!(client.get_pull_request(1).unwrap().title, "New");
        assert_eq!(client.get_pull_request(1).unwrap().description, "old desc");
    }

    #[test]
    fn test_update_pull_request_nonexistent() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.update_pull_request(999, Some("x"), None));
    }

    #[test]
    fn test_close_pull_request() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR", "", "f", "m");
        assert!(client.close_pull_request(1));
        assert_eq!(client.get_pull_request(1).unwrap().state, PrState::Closed);
    }

    #[test]
    fn test_close_nonexistent_pr() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.close_pull_request(1));
    }

    #[test]
    fn test_merge_pull_request() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR", "", "f", "m");
        assert!(client.merge_pull_request(1, MergeMethod::Squash));
        let pr = client.get_pull_request(1).unwrap();
        assert_eq!(pr.state, PrState::Merged);
        assert_eq!(pr.merge_method, Some(MergeMethod::Squash));
    }

    #[test]
    fn test_merge_closed_pr_fails() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR", "", "f", "m");
        client.close_pull_request(1);
        assert!(!client.merge_pull_request(1, MergeMethod::Merge));
    }

    #[test]
    fn test_add_reviewer() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR", "", "f", "m");
        assert!(client.add_reviewer(1, "alice"));
        assert!(client.add_reviewer(1, "bob"));
        // duplicate should not add twice
        client.add_reviewer(1, "alice");
        assert_eq!(client.get_pull_request(1).unwrap().reviewers.len(), 2);
    }

    #[test]
    fn test_add_reviewer_nonexistent_pr() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.add_reviewer(99, "alice"));
    }

    #[test]
    fn test_add_label() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("PR", "", "f", "m");
        assert!(client.add_label(1, "bug"));
        assert!(client.add_label(1, "urgent"));
        client.add_label(1, "bug"); // dup
        assert_eq!(client.get_pull_request(1).unwrap().labels.len(), 2);
    }

    #[test]
    fn test_pr_increments_id() {
        let mut client = GitPlatformClient::new(github_config());
        let pr1 = client.create_pull_request("A", "", "a", "m");
        let pr2 = client.create_pull_request("B", "", "b", "m");
        assert_eq!(pr1.number, 1);
        assert_eq!(pr2.number, 2);
    }

    // --- Issue CRUD tests ---

    #[test]
    fn test_create_issue() {
        let mut client = GitPlatformClient::new(github_config());
        let issue = client.create_issue("Bug report", "It's broken");
        assert_eq!(issue.title, "Bug report");
        assert_eq!(issue.state, IssueState::Open);
        assert_eq!(issue.number, 1);
    }

    #[test]
    fn test_get_issue() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("Bug", "desc");
        assert!(client.get_issue(1).is_some());
        assert!(client.get_issue(99).is_none());
    }

    #[test]
    fn test_list_issues_all() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("A", "");
        client.create_issue("B", "");
        assert_eq!(client.list_issues(None).len(), 2);
    }

    #[test]
    fn test_list_issues_by_state() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("A", "");
        client.create_issue("B", "");
        client.close_issue(1);
        assert_eq!(client.list_issues(Some(&IssueState::Open)).len(), 1);
        assert_eq!(client.list_issues(Some(&IssueState::Closed)).len(), 1);
    }

    #[test]
    fn test_close_issue() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("Bug", "");
        assert!(client.close_issue(1));
        assert_eq!(client.get_issue(1).unwrap().state, IssueState::Closed);
    }

    #[test]
    fn test_close_nonexistent_issue() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.close_issue(99));
    }

    #[test]
    fn test_assign_issue() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("Bug", "");
        assert!(client.assign_issue(1, "dev1"));
        assert!(client.assign_issue(1, "dev2"));
        client.assign_issue(1, "dev1"); // dup
        assert_eq!(client.get_issue(1).unwrap().assignees.len(), 2);
    }

    #[test]
    fn test_assign_nonexistent_issue() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.assign_issue(99, "dev"));
    }

    // --- Pipeline tests ---

    #[test]
    fn test_add_and_get_pipeline() {
        let mut client = GitPlatformClient::new(github_config());
        let pipe = Pipeline {
            id: 1,
            name: "CI".to_string(),
            status: PipelineStatus::Running,
            branch: "main".to_string(),
            commit_sha: "abc".to_string(),
            stages: vec![],
            started_at: Some(SystemTime::now()),
            finished_at: None,
            url: "https://ci.example.com/1".to_string(),
        };
        client.add_pipeline(pipe);
        assert!(client.get_pipeline(1).is_some());
        assert!(client.get_pipeline(99).is_none());
    }

    #[test]
    fn test_list_pipelines() {
        let mut client = GitPlatformClient::new(github_config());
        client.add_pipeline(Pipeline {
            id: 1, name: "A".into(), status: PipelineStatus::Success,
            branch: "main".into(), commit_sha: "a".into(), stages: vec![],
            started_at: None, finished_at: None, url: String::new(),
        });
        client.add_pipeline(Pipeline {
            id: 2, name: "B".into(), status: PipelineStatus::Failed,
            branch: "dev".into(), commit_sha: "b".into(), stages: vec![],
            started_at: None, finished_at: None, url: String::new(),
        });
        assert_eq!(client.list_pipelines().len(), 2);
    }

    #[test]
    fn test_latest_pipeline_for_branch() {
        let mut client = GitPlatformClient::new(github_config());
        client.add_pipeline(Pipeline {
            id: 1, name: "CI".into(), status: PipelineStatus::Success,
            branch: "main".into(), commit_sha: "a".into(), stages: vec![],
            started_at: None, finished_at: None, url: String::new(),
        });
        client.add_pipeline(Pipeline {
            id: 2, name: "CI".into(), status: PipelineStatus::Running,
            branch: "main".into(), commit_sha: "b".into(), stages: vec![],
            started_at: None, finished_at: None, url: String::new(),
        });
        let latest = client.latest_pipeline_for_branch("main").unwrap();
        assert_eq!(latest.id, 2);
        assert!(client.latest_pipeline_for_branch("nonexistent").is_none());
    }

    // --- Branch tests ---

    #[test]
    fn test_create_branch() {
        let mut client = GitPlatformClient::new(github_config());
        let b = client.create_branch("feature-x", "abc123");
        assert_eq!(b.name, "feature-x");
        assert_eq!(b.sha, "abc123");
        assert!(!b.is_protected);
        assert!(!b.is_default);
    }

    #[test]
    fn test_list_branches() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_branch("a", "sha1");
        client.create_branch("b", "sha2");
        assert_eq!(client.list_branches().len(), 2);
    }

    #[test]
    fn test_delete_branch() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_branch("temp", "sha");
        assert!(client.delete_branch("temp"));
        assert_eq!(client.list_branches().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_branch() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.delete_branch("ghost"));
    }

    #[test]
    fn test_get_default_branch() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_branch("dev", "sha1");
        assert!(client.get_default_branch().is_none());
        client.branches.push(BranchInfo {
            name: "main".into(),
            sha: "sha2".into(),
            is_protected: true,
            is_default: true,
        });
        assert_eq!(client.get_default_branch().unwrap().name, "main");
    }

    // --- Webhook tests ---

    #[test]
    fn test_create_webhook() {
        let mut client = GitPlatformClient::new(github_config());
        let wh = client.create_webhook("https://hook.example.com", vec![WebhookEvent::Push, WebhookEvent::PullRequest]);
        assert_eq!(wh.url, "https://hook.example.com");
        assert_eq!(wh.events.len(), 2);
        assert!(wh.active);
        assert!(wh.id.starts_with("wh-"));
    }

    #[test]
    fn test_list_webhooks() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_webhook("https://a.com", vec![WebhookEvent::Push]);
        client.create_webhook("https://b.com", vec![WebhookEvent::Issue]);
        assert_eq!(client.list_webhooks().len(), 2);
    }

    #[test]
    fn test_delete_webhook() {
        let mut client = GitPlatformClient::new(github_config());
        let wh = client.create_webhook("https://a.com", vec![]);
        assert!(client.delete_webhook(&wh.id));
        assert_eq!(client.list_webhooks().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_webhook() {
        let mut client = GitPlatformClient::new(github_config());
        assert!(!client.delete_webhook("wh-999"));
    }

    // --- Status check tests ---

    #[test]
    fn test_create_status_check() {
        let client = GitPlatformClient::new(github_config());
        let sc = client.create_status_check("sha1", "ci/test", PipelineStatus::Success, "All passed");
        assert_eq!(sc.context, "ci/test");
        assert_eq!(sc.state, PipelineStatus::Success);
        assert_eq!(sc.description, "All passed");
        assert!(sc.target_url.is_some());
    }

    // --- Statistics tests ---

    #[test]
    fn test_open_pr_count() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_pull_request("A", "", "a", "m");
        client.create_pull_request("B", "", "b", "m");
        client.close_pull_request(1);
        assert_eq!(client.open_pr_count(), 1);
    }

    #[test]
    fn test_open_issue_count() {
        let mut client = GitPlatformClient::new(github_config());
        client.create_issue("A", "");
        client.create_issue("B", "");
        client.create_issue("C", "");
        client.close_issue(2);
        assert_eq!(client.open_issue_count(), 2);
    }

    #[test]
    fn test_recent_commits_returns_empty() {
        let client = GitPlatformClient::new(github_config());
        assert!(client.recent_commits("main").is_empty());
    }

    // --- Platform name tests ---

    #[test]
    fn test_platform_name_github() {
        let client = GitPlatformClient::new(github_config());
        assert_eq!(client.platform_name(), "GitHub");
    }

    #[test]
    fn test_platform_name_gitlab() {
        let client = GitPlatformClient::new(gitlab_config());
        assert_eq!(client.platform_name(), "GitLab");
    }

    #[test]
    fn test_platform_name_azure() {
        let client = GitPlatformClient::new(azure_config());
        assert_eq!(client.platform_name(), "Azure DevOps");
    }

    #[test]
    fn test_platform_name_custom() {
        let cfg = PlatformConfig {
            platform: GitPlatform::Custom("Forgejo".to_string()),
            base_url: "https://forgejo.local".to_string(),
            auth: AuthMethod::Token("t".to_string()),
            default_branch: "main".to_string(),
            project_path: "u/r".to_string(),
            api_version: None,
        };
        let client = GitPlatformClient::new(cfg);
        assert_eq!(client.platform_name(), "Forgejo");
    }

    // --- api_urls integration test ---

    #[test]
    fn test_api_urls_from_client() {
        let client = GitPlatformClient::new(github_config());
        let urls = client.api_urls();
        assert!(urls.pull_requests_url().contains("/pulls"));
        assert!(urls.issues_url().contains("/issues"));
    }

    // --- Multi-Platform Manager tests ---

    #[test]
    fn test_manager_new_empty() {
        let mgr = PlatformManager::new();
        assert!(mgr.clients.is_empty());
        assert!(mgr.default_platform.is_none());
    }

    #[test]
    fn test_manager_add_platform_sets_default() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        assert_eq!(mgr.default_platform.as_deref(), Some("gh"));
    }

    #[test]
    fn test_manager_add_second_platform_keeps_first_default() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        mgr.add_platform("gl", gitlab_config());
        assert_eq!(mgr.default_platform.as_deref(), Some("gh"));
    }

    #[test]
    fn test_manager_get_platform() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        assert!(mgr.get_platform("gh").is_some());
        assert!(mgr.get_platform("missing").is_none());
    }

    #[test]
    fn test_manager_get_platform_mut() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        let client = mgr.get_platform_mut("gh").unwrap();
        client.create_pull_request("Test", "", "f", "m");
        assert_eq!(mgr.get_platform("gh").unwrap().pull_requests.len(), 1);
    }

    #[test]
    fn test_manager_set_default() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        mgr.add_platform("gl", gitlab_config());
        assert!(mgr.set_default("gl"));
        assert_eq!(mgr.default_platform.as_deref(), Some("gl"));
    }

    #[test]
    fn test_manager_set_default_nonexistent() {
        let mut mgr = PlatformManager::new();
        assert!(!mgr.set_default("nope"));
    }

    #[test]
    fn test_manager_default_client() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        assert_eq!(mgr.default_client().unwrap().platform_name(), "GitHub");
    }

    #[test]
    fn test_manager_default_client_none() {
        let mgr = PlatformManager::new();
        assert!(mgr.default_client().is_none());
    }

    #[test]
    fn test_manager_default_client_mut() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        let client = mgr.default_client_mut().unwrap();
        client.create_issue("Test", "desc");
        assert_eq!(mgr.default_client().unwrap().issues.len(), 1);
    }

    #[test]
    fn test_manager_list_platforms() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        mgr.add_platform("gl", gitlab_config());
        let list = mgr.list_platforms();
        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"gh"));
        assert!(names.contains(&"gl"));
    }

    #[test]
    fn test_manager_remove_platform() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        mgr.add_platform("gl", gitlab_config());
        assert!(mgr.remove_platform("gh"));
        assert!(mgr.get_platform("gh").is_none());
        // default should switch to remaining platform
        assert!(mgr.default_platform.is_some());
    }

    #[test]
    fn test_manager_remove_nonexistent() {
        let mut mgr = PlatformManager::new();
        assert!(!mgr.remove_platform("ghost"));
    }

    #[test]
    fn test_manager_remove_default_reassigns() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("a", github_config());
        mgr.add_platform("b", gitlab_config());
        mgr.set_default("a");
        mgr.remove_platform("a");
        // default should have been reassigned
        assert!(mgr.default_platform.is_some());
        assert_ne!(mgr.default_platform.as_deref(), Some("a"));
    }

    #[test]
    fn test_sync_pr_across_platforms() {
        let mut mgr = PlatformManager::new();
        mgr.add_platform("gh", github_config());
        mgr.add_platform("gl", gitlab_config());
        mgr.add_platform("az", azure_config());

        let pr = PullRequest {
            id: 1, number: 1, title: "Test".into(), description: "Desc".into(),
            source_branch: "f".into(), target_branch: "m".into(),
            state: PrState::Open, author: "u".into(),
            reviewers: vec![], labels: vec![],
            created_at: SystemTime::now(), updated_at: SystemTime::now(),
            merge_method: None, ci_status: None, url: String::new(),
            diff_stats: DiffStats { files_changed: 0, additions: 0, deletions: 0 },
        };

        let synced = mgr.sync_pr_across_platforms(&pr, &["gl", "az", "nonexistent"]);
        assert_eq!(synced.len(), 2);
        assert!(synced.contains(&"gl".to_string()));
        assert!(synced.contains(&"az".to_string()));
    }

    #[test]
    fn test_sync_pr_empty_targets() {
        let mgr = PlatformManager::new();
        let pr = PullRequest {
            id: 1, number: 1, title: "T".into(), description: "".into(),
            source_branch: "f".into(), target_branch: "m".into(),
            state: PrState::Open, author: "u".into(),
            reviewers: vec![], labels: vec![],
            created_at: SystemTime::now(), updated_at: SystemTime::now(),
            merge_method: None, ci_status: None, url: String::new(),
            diff_stats: DiffStats { files_changed: 0, additions: 0, deletions: 0 },
        };
        assert!(mgr.sync_pr_across_platforms(&pr, &[]).is_empty());
    }

    // --- Edge case tests ---

    #[test]
    fn test_trailing_slash_stripped_in_url_builder() {
        let b = ApiUrlBuilder::new(GitPlatform::GitHub, "https://api.github.com/", "o/r");
        assert!(!b.pull_requests_url().contains("//repos"));
    }

    #[test]
    fn test_gitlab_config_trailing_slash_base_url() {
        let cfg = PlatformConfig::gitlab("t", "https://gl.com/", "g/p");
        assert_eq!(cfg.base_url, "https://gl.com/api/v4");
    }

    #[test]
    fn test_gitea_config_trailing_slash_base_url() {
        let cfg = PlatformConfig::gitea("t", "https://gitea.local/", "u/r");
        assert_eq!(cfg.base_url, "https://gitea.local/api/v1");
    }

    #[test]
    fn test_auth_method_variants() {
        let _token = AuthMethod::Token("tok".into());
        let _oauth = AuthMethod::OAuth("oa".into());
        let _app = AuthMethod::App { app_id: "123".into(), private_key: "pk".into() };
        let _basic = AuthMethod::Basic { username: "u".into(), password: "p".into() };
    }

    #[test]
    fn test_pipeline_stage_with_jobs() {
        let stage = PipelineStage {
            name: "build".into(),
            status: PipelineStatus::Success,
            jobs: vec![
                PipelineJob { name: "compile".into(), status: PipelineStatus::Success, log_url: Some("url".into()), duration_secs: Some(30) },
                PipelineJob { name: "lint".into(), status: PipelineStatus::Success, log_url: None, duration_secs: Some(10) },
            ],
            duration_secs: Some(40),
        };
        assert_eq!(stage.jobs.len(), 2);
        assert_eq!(stage.duration_secs, Some(40));
    }

    #[test]
    fn test_review_with_comments() {
        let review = Review {
            id: 1,
            author: "reviewer".into(),
            state: ReviewState::ChangesRequested,
            body: "Please fix".into(),
            comments: vec![
                ReviewComment { file_path: "src/main.rs".into(), line: 42, body: "typo".into(), author: "reviewer".into() },
            ],
            submitted_at: SystemTime::now(),
        };
        assert_eq!(review.comments.len(), 1);
        assert_eq!(review.state, ReviewState::ChangesRequested);
    }

    #[test]
    fn test_diff_stats() {
        let stats = DiffStats { files_changed: 5, additions: 100, deletions: 30 };
        assert_eq!(stats.files_changed, 5);
        assert_eq!(stats.additions, 100);
        assert_eq!(stats.deletions, 30);
    }

    #[test]
    fn test_repository_struct() {
        let repo = Repository {
            id: 1, name: "test".into(), full_path: "user/test".into(),
            default_branch: "main".into(), description: Some("A repo".into()),
            visibility: "public".into(), clone_url: "https://github.com/user/test.git".into(),
            web_url: "https://github.com/user/test".into(),
            created_at: SystemTime::now(), language: Some("Rust".into()),
            stars: 42, forks: 7,
        };
        assert_eq!(repo.name, "test");
        assert_eq!(repo.stars, 42);
    }

    #[test]
    fn test_commit_info_struct() {
        let ci = CommitInfo {
            sha: "abc123".into(), message: "fix: bug".into(),
            author: "dev".into(), author_email: "dev@example.com".into(),
            timestamp: SystemTime::now(),
            files_changed: vec!["src/lib.rs".into(), "tests/test.rs".into()],
        };
        assert_eq!(ci.files_changed.len(), 2);
    }

    #[test]
    fn test_webhook_events_all_variants() {
        let events = vec![
            WebhookEvent::Push, WebhookEvent::PullRequest, WebhookEvent::Issue,
            WebhookEvent::Comment, WebhookEvent::PipelineComplete,
            WebhookEvent::TagCreated, WebhookEvent::BranchCreated,
            WebhookEvent::BranchDeleted, WebhookEvent::Release,
        ];
        assert_eq!(events.len(), 9);
    }

    #[test]
    fn test_gitlab_url_builder_status_checks() {
        let b = ApiUrlBuilder::new(GitPlatform::GitLab, "https://gl.com/api/v4", "g/p");
        let url = b.status_checks_url("deadbeef");
        assert!(url.contains("g%2Fp"));
        assert!(url.contains("deadbeef"));
        assert!(url.contains("statuses"));
    }

    #[test]
    fn test_azure_repository_url() {
        let b = ApiUrlBuilder::new(GitPlatform::AzureDevOps, "https://dev.azure.com", "org/proj/repo");
        let url = b.repository_url();
        assert!(url.contains("org"));
        assert!(url.contains("proj"));
        assert!(url.contains("repo"));
        assert!(url.contains("_apis/git/repositories"));
    }

    #[test]
    fn test_bitbucket_status_checks_url() {
        let b = ApiUrlBuilder::new(GitPlatform::Bitbucket, "https://api.bitbucket.org/2.0", "ws/repo");
        let url = b.status_checks_url("sha1");
        assert_eq!(url, "https://api.bitbucket.org/2.0/repositories/ws/repo/commit/sha1/statuses");
    }

    #[test]
    fn test_custom_platform_url_builder() {
        let b = ApiUrlBuilder::new(GitPlatform::Custom("Forgejo".into()), "https://forgejo.local/api/v1", "u/r");
        assert!(b.pull_requests_url().contains("/repos/u/r/pulls"));
        assert!(b.issues_url().contains("/repos/u/r/issues"));
        assert!(b.repository_url().contains("/repos/u/r"));
    }
}
