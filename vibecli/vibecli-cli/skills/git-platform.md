---
triggers: ["gitlab", "azure devops", "bitbucket", "gitea", "git platform", "multi-platform git", "gitlab integration", "azure repos", "bitbucket integration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Multi-Platform Git Integration

When working with Git platforms beyond GitHub:

1. Configure platform connections: use PlatformConfig factory methods for each platform — github(token, repo), gitlab(token, base_url, project), azure_devops(token, org, project, repo), bitbucket(token, workspace, repo), gitea(token, base_url, repo). Store configs in ~/.vibecli/config.toml under [git_platforms].
2. Use platform-specific API URLs: GitHub (/repos/{owner}/{repo}/...), GitLab (/api/v4/projects/{id}/...), Azure DevOps (/{org}/{project}/_apis/git/repositories/{repo}/...), Bitbucket (/2.0/repositories/{workspace}/{repo}/...). The ApiUrlBuilder handles these differences automatically.
3. Create PRs/MRs uniformly: use create_pull_request(title, description, source_branch, target_branch) regardless of platform. GitLab calls them "merge requests" but the API is unified.
4. Track CI/CD pipelines across platforms: GitHub Actions, GitLab CI, Azure Pipelines, Bitbucket Pipelines all have different status models. The Pipeline struct normalizes stages, jobs, and statuses.
5. Manage webhooks consistently: create_webhook(url, events) works across all platforms. Events (Push, PullRequest, Issue, PipelineComplete, etc.) are mapped to platform-specific event names.
6. Set a default platform: when multiple platforms are configured, set_default() determines which platform is used for unqualified commands. Use platform-prefixed commands for specific platforms.
7. Sync PRs across platforms: sync_pr_across_platforms() mirrors a PR from one platform to others — useful for mirrored repositories or multi-platform organizations.
8. Query status checks: create_status_check() posts CI/AI review results as commit statuses. Works with GitHub status checks, GitLab commit statuses, Azure DevOps build results.
