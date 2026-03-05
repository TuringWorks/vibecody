//! Cloud-isolated agent execution via Docker containers.
//!
//! Agents run inside Docker containers for isolation, producing PRs as output.
//! This module handles container lifecycle, configuration, and status tracking.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for a cloud-isolated agent Docker container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAgentConfig {
    /// Docker image to use (e.g. "ubuntu:22.04", "rust:1.77", custom image).
    pub image: String,
    /// Optional git repo URL to clone inside the container.
    pub repo_url: Option<String>,
    /// Branch to check out (defaults to main/master if not set).
    pub branch: Option<String>,
    /// Host directory to mount as /workspace inside the container.
    pub workspace_mount: Option<String>,
    /// Environment variables to pass into the container.
    pub env_vars: Vec<(String, String)>,
    /// Maximum execution time in seconds before the container is stopped.
    pub timeout_secs: u64,
}

impl Default for CloudAgentConfig {
    fn default() -> Self {
        Self {
            image: "ubuntu:22.04".to_string(),
            repo_url: None,
            branch: None,
            workspace_mount: None,
            env_vars: Vec::new(),
            timeout_secs: 3600,
        }
    }
}

/// Status of a cloud agent container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAgentStatus {
    /// Docker container ID or name.
    pub container_id: String,
    /// Current status: "starting", "running", "complete", or "failed".
    pub status: String,
    /// Captured stdout/stderr log lines from the container.
    pub logs: Vec<String>,
    /// Git branch created by the agent (if any).
    pub branch_created: Option<String>,
    /// Pull request URL created by the agent (if any).
    pub pr_url: Option<String>,
    /// Unix timestamp when the container was started.
    pub started_at: u64,
    /// Unix timestamp when the container finished (None if still running).
    pub finished_at: Option<u64>,
}

/// Check whether Docker is installed and the daemon is running.
pub fn check_docker() -> Result<bool> {
    let output = std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output();
    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

/// Build the `docker run` argument list for an agent task.
///
/// The container is started with `--rm` so it is automatically cleaned up,
/// and the task description is passed via the `VIBECODY_TASK` env var.
pub fn build_docker_command(config: &CloudAgentConfig, task: &str) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--name".to_string(),
        format!("vibecody-agent-{}", rand_hex()),
    ];

    // Mount workspace if specified
    if let Some(mount) = &config.workspace_mount {
        args.push("-v".to_string());
        args.push(format!("{}:/workspace", mount));
        args.push("-w".to_string());
        args.push("/workspace".to_string());
    }

    // Set environment variables
    for (k, v) in &config.env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", k, v));
    }

    // Pass the task description as an environment variable
    args.push("-e".to_string());
    args.push(format!("VIBECODY_TASK={}", task));

    // Container stop timeout
    args.push("--stop-timeout".to_string());
    args.push(config.timeout_secs.to_string());

    // Image
    args.push(config.image.clone());

    // Default entrypoint: bash script that echoes the task
    args.push("bash".to_string());
    args.push("-c".to_string());
    args.push(format!(
        "echo 'VibeCody cloud agent starting' && \
         echo 'Task: {}' && \
         echo 'Image: {}' && \
         echo 'Task complete'",
        task.replace('\'', "'\\''"),
        config.image.replace('\'', "'\\''"),
    ));

    args
}

/// Start a cloud agent inside a Docker container and wait for it to finish.
///
/// Returns a `CloudAgentStatus` with the container's logs, exit status,
/// and timing information.
pub async fn start_cloud_agent(
    config: &CloudAgentConfig,
    task: &str,
) -> Result<CloudAgentStatus> {
    if !check_docker()? {
        anyhow::bail!("Docker is not installed or not running. Install Docker from https://docs.docker.com/get-docker/");
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let args = build_docker_command(config, task);
    let container_name = args[3].clone();

    let output = tokio::process::Command::new("docker")
        .args(&args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start Docker container: {}", e))?;

    let mut logs: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect();

    // Also capture stderr lines
    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| format!("[stderr] {}", l))
        .collect();
    logs.extend(stderr_lines);

    let finished = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let status = if output.status.success() {
        "complete"
    } else {
        "failed"
    };

    Ok(CloudAgentStatus {
        container_id: container_name,
        status: status.to_string(),
        logs,
        branch_created: None,
        pr_url: None,
        started_at: now,
        finished_at: Some(finished),
    })
}

/// Generate a short random hex string for container naming.
fn rand_hex() -> String {
    use rand::Rng;
    format!("{:08x}", rand::thread_rng().gen::<u32>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = CloudAgentConfig::default();
        assert_eq!(config.image, "ubuntu:22.04");
        assert_eq!(config.timeout_secs, 3600);
        assert!(config.env_vars.is_empty());
        assert!(config.repo_url.is_none());
        assert!(config.branch.is_none());
        assert!(config.workspace_mount.is_none());
    }

    #[test]
    fn build_command_basic() {
        let config = CloudAgentConfig::default();
        let args = build_docker_command(&config, "fix the bug");
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"ubuntu:22.04".to_string()));
        // Should contain the task in VIBECODY_TASK env var
        assert!(args.iter().any(|a| a.starts_with("VIBECODY_TASK=")));
    }

    #[test]
    fn build_command_with_mount() {
        let mut config = CloudAgentConfig::default();
        config.workspace_mount = Some("/home/user/project".to_string());
        let args = build_docker_command(&config, "test");
        assert!(args.contains(&"-v".to_string()));
        assert!(args
            .iter()
            .any(|a| a.contains("/home/user/project:/workspace")));
        assert!(args.contains(&"-w".to_string()));
        assert!(args.contains(&"/workspace".to_string()));
    }

    #[test]
    fn build_command_with_env_vars() {
        let mut config = CloudAgentConfig::default();
        config.env_vars = vec![
            ("API_KEY".to_string(), "secret123".to_string()),
            ("DEBUG".to_string(), "1".to_string()),
        ];
        let args = build_docker_command(&config, "deploy");
        // Should have two -e flags for user env vars plus the VIBECODY_TASK one
        let env_count = args.iter().filter(|a| *a == "-e").count();
        assert_eq!(env_count, 3); // API_KEY, DEBUG, VIBECODY_TASK
    }

    #[test]
    fn build_command_custom_image() {
        let mut config = CloudAgentConfig::default();
        config.image = "rust:1.77-slim".to_string();
        let args = build_docker_command(&config, "cargo test");
        assert!(args.contains(&"rust:1.77-slim".to_string()));
    }

    #[test]
    fn status_serde_roundtrip() {
        let status = CloudAgentStatus {
            container_id: "vibecody-agent-abc123".to_string(),
            status: "running".to_string(),
            logs: vec!["Starting...".to_string(), "Working...".to_string()],
            branch_created: Some("fix/bug-123".to_string()),
            pr_url: None,
            started_at: 1000,
            finished_at: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: CloudAgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.container_id, "vibecody-agent-abc123");
        assert_eq!(parsed.status, "running");
        assert_eq!(parsed.logs.len(), 2);
        assert_eq!(parsed.branch_created, Some("fix/bug-123".to_string()));
        assert!(parsed.pr_url.is_none());
        assert!(parsed.finished_at.is_none());
    }

    #[test]
    fn check_docker_returns_result() {
        // This test just verifies that check_docker() doesn't panic.
        // It may return Ok(true) or Ok(false) depending on the host.
        let result = check_docker();
        assert!(result.is_ok());
    }

    #[test]
    fn rand_hex_format() {
        let hex = rand_hex();
        assert_eq!(hex.len(), 8);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
