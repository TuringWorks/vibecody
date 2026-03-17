#![allow(dead_code)]
//! Container Runtime Abstraction Layer
//!
//! Provides a unified async trait for Docker, Podman, and OpenSandbox container runtimes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Which container runtime is backing this execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeKind {
    Docker,
    Podman,
    OpenSandbox,
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::Podman => write!(f, "podman"),
            Self::OpenSandbox => write!(f, "opensandbox"),
        }
    }
}

impl std::str::FromStr for RuntimeKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(Self::Docker),
            "podman" => Ok(Self::Podman),
            "opensandbox" => Ok(Self::OpenSandbox),
            _ => Err(anyhow::anyhow!("unknown runtime: {s}")),
        }
    }
}

/// Resource limits applied to a container.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    /// CPU cores (e.g. 2.0 = two cores).
    pub cpus: Option<f64>,
    /// Memory in bytes.
    pub memory_bytes: Option<u64>,
    /// Maximum number of PIDs.
    pub pids_limit: Option<u32>,
}

/// Network access policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum NetworkPolicy {
    /// No network access.
    None,
    /// Only allow egress to specific domains.
    Restricted { allowed_domains: Vec<String> },
    /// Full unrestricted network.
    #[default]
    Full,
}

/// A host→container volume mount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// Configuration for creating a new container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container image (e.g. "ubuntu:22.04").
    pub image: String,
    /// Optional container name.
    pub name: Option<String>,
    /// Environment variables.
    pub env: Vec<(String, String)>,
    /// Volume mounts.
    pub volumes: Vec<VolumeMount>,
    /// Resource limits.
    pub resource_limits: ResourceLimits,
    /// Network policy.
    pub network_policy: NetworkPolicy,
    /// Auto-kill after N seconds (0 = no timeout).
    pub timeout_secs: u64,
    /// Working directory inside the container.
    pub working_dir: Option<String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            image: "ubuntu:22.04".to_string(),
            name: None,
            env: vec![],
            volumes: vec![],
            resource_limits: ResourceLimits::default(),
            network_policy: NetworkPolicy::Full,
            timeout_secs: 3600,
            working_dir: Some("/workspace".to_string()),
        }
    }
}

/// Information about a running or stopped container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at: String,
    pub runtime: RuntimeKind,
}

/// Result of executing a command inside a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Streaming events from a container exec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecStreamEvent {
    Stdout(String),
    Stderr(String),
    ExitCode(i32),
    Error(String),
}

/// Runtime resource metrics for a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    pub cpu_usage_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: u64,
    pub pids: u32,
}

/// Unified async interface for all container runtimes.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Which runtime kind this is.
    fn kind(&self) -> RuntimeKind;

    /// Check if the runtime binary/service is available.
    async fn is_available(&self) -> bool;

    /// Get the runtime version string.
    async fn version(&self) -> anyhow::Result<String>;

    // ── Lifecycle ────────────────────────────────────────────────────────────

    /// Create and start a new container.
    async fn create(&self, config: &ContainerConfig) -> anyhow::Result<ContainerInfo>;

    /// Stop a running container.
    async fn stop(&self, id: &str) -> anyhow::Result<()>;

    /// Remove a stopped container.
    async fn remove(&self, id: &str) -> anyhow::Result<()>;

    /// Pause a running container.
    async fn pause(&self, id: &str) -> anyhow::Result<()>;

    /// Resume a paused container.
    async fn resume(&self, id: &str) -> anyhow::Result<()>;

    /// List all VibeCody-managed containers.
    async fn list(&self) -> anyhow::Result<Vec<ContainerInfo>>;

    /// Inspect a specific container.
    async fn inspect(&self, id: &str) -> anyhow::Result<ContainerInfo>;

    // ── Execution ────────────────────────────────────────────────────────────

    /// Execute a command and wait for completion.
    async fn exec(&self, id: &str, command: &str, cwd: Option<&str>) -> anyhow::Result<ExecResult>;

    /// Execute a command with streaming output.
    async fn exec_stream(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
        tx: mpsc::Sender<ExecStreamEvent>,
    ) -> anyhow::Result<()>;

    // ── File Operations ──────────────────────────────────────────────────────

    /// Read a file from the container.
    async fn read_file(&self, id: &str, path: &str) -> anyhow::Result<String>;

    /// Write content to a file inside the container.
    async fn write_file(&self, id: &str, path: &str, content: &str) -> anyhow::Result<()>;

    /// List directory contents.
    async fn list_dir(&self, id: &str, path: &str) -> anyhow::Result<Vec<String>>;

    // ── Monitoring ───────────────────────────────────────────────────────────

    /// Get container logs.
    async fn logs(&self, id: &str, tail: Option<u32>) -> anyhow::Result<String>;

    /// Get container resource metrics.
    async fn metrics(&self, id: &str) -> anyhow::Result<ContainerMetrics>;
}

/// Auto-detect the best available container runtime.
pub async fn detect_runtime(
    config: &crate::config::SandboxConfig,
) -> anyhow::Result<Box<dyn ContainerRuntime>> {
    let preferred = &config.runtime;

    match preferred.as_str() {
        "docker" => {
            let rt = crate::docker_runtime::DockerRuntime::new();
            if rt.is_available().await {
                return Ok(Box::new(rt));
            }
            anyhow::bail!("Docker requested but not available");
        }
        "podman" => {
            let rt = crate::podman_runtime::PodmanRuntime::new();
            if rt.is_available().await {
                return Ok(Box::new(rt));
            }
            anyhow::bail!("Podman requested but not available");
        }
        "opensandbox" => {
            let api_url = config.opensandbox.resolve_api_url();
            let api_key = config.opensandbox.resolve_api_key();
            let rt = crate::opensandbox_client::OpenSandboxRuntime::new(api_url, api_key);
            if rt.is_available().await {
                return Ok(Box::new(rt));
            }
            anyhow::bail!("OpenSandbox requested but not available");
        }
        _ => {
            // Try Docker first, then Podman, then OpenSandbox
            let docker = crate::docker_runtime::DockerRuntime::new();
            if docker.is_available().await {
                return Ok(Box::new(docker));
            }

            let podman = crate::podman_runtime::PodmanRuntime::new();
            if podman.is_available().await {
                return Ok(Box::new(podman));
            }

            let api_url = config.opensandbox.resolve_api_url();
            let api_key = config.opensandbox.resolve_api_key();
            let osb = crate::opensandbox_client::OpenSandboxRuntime::new(api_url, api_key);
            if osb.is_available().await {
                return Ok(Box::new(osb));
            }

            anyhow::bail!(
                "No container runtime found. Install Docker, Podman, or configure OpenSandbox."
            );
        }
    }
}

/// Generate a unique container name with the vibecody prefix.
pub fn generate_container_name() -> String {
    let id = uuid::Uuid::new_v4();
    let hex = &id.to_string().replace('-', "")[..12];
    format!("vibecody-sb-{hex}")
}

/// Parse a human-friendly memory string like "4g" or "512m" into bytes.
pub fn parse_memory_string(s: &str) -> anyhow::Result<u64> {
    let s = s.trim().to_lowercase();
    if let Some(num) = s.strip_suffix("g") {
        let n: f64 = num.parse()?;
        Ok((n * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = s.strip_suffix("m") {
        let n: f64 = num.parse()?;
        Ok((n * 1024.0 * 1024.0) as u64)
    } else if let Some(num) = s.strip_suffix("k") {
        let n: f64 = num.parse()?;
        Ok((n * 1024.0) as u64)
    } else {
        Ok(s.parse::<u64>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_kind_display() {
        assert_eq!(RuntimeKind::Docker.to_string(), "docker");
        assert_eq!(RuntimeKind::Podman.to_string(), "podman");
        assert_eq!(RuntimeKind::OpenSandbox.to_string(), "opensandbox");
    }

    #[test]
    fn runtime_kind_from_str() {
        assert_eq!("docker".parse::<RuntimeKind>().unwrap(), RuntimeKind::Docker);
        assert_eq!("Podman".parse::<RuntimeKind>().unwrap(), RuntimeKind::Podman);
        assert_eq!(
            "opensandbox".parse::<RuntimeKind>().unwrap(),
            RuntimeKind::OpenSandbox
        );
        assert!("invalid".parse::<RuntimeKind>().is_err());
    }

    #[test]
    fn container_name_format() {
        let name = generate_container_name();
        assert!(name.starts_with("vibecody-sb-"));
        assert_eq!(name.len(), "vibecody-sb-".len() + 12);
    }

    #[test]
    fn parse_memory_gigabytes() {
        assert_eq!(parse_memory_string("4g").unwrap(), 4 * 1024 * 1024 * 1024);
        assert_eq!(parse_memory_string("1G").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_megabytes() {
        assert_eq!(parse_memory_string("512m").unwrap(), 512 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_kilobytes() {
        assert_eq!(parse_memory_string("1024k").unwrap(), 1024 * 1024);
    }

    #[test]
    fn parse_memory_bytes() {
        assert_eq!(parse_memory_string("1048576").unwrap(), 1048576);
    }

    #[test]
    fn parse_memory_invalid() {
        assert!(parse_memory_string("abc").is_err());
    }

    #[test]
    fn default_container_config() {
        let cfg = ContainerConfig::default();
        assert_eq!(cfg.image, "ubuntu:22.04");
        assert_eq!(cfg.timeout_secs, 3600);
        assert_eq!(cfg.working_dir, Some("/workspace".to_string()));
    }

    #[test]
    fn default_network_policy_is_full() {
        let policy = NetworkPolicy::default();
        assert!(matches!(policy, NetworkPolicy::Full));
    }

    #[test]
    fn container_name_uniqueness() {
        let name1 = generate_container_name();
        let name2 = generate_container_name();
        assert_ne!(name1, name2);
    }

    #[test]
    fn parse_memory_with_whitespace() {
        assert_eq!(parse_memory_string("  2g  ").unwrap(), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_memory_string(" 512m ").unwrap(), 512 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_case_insensitive() {
        assert_eq!(parse_memory_string("4G").unwrap(), parse_memory_string("4g").unwrap());
        assert_eq!(parse_memory_string("512M").unwrap(), parse_memory_string("512m").unwrap());
        assert_eq!(parse_memory_string("1024K").unwrap(), parse_memory_string("1024k").unwrap());
    }

    #[test]
    fn runtime_kind_serde_roundtrip() {
        for kind in [RuntimeKind::Docker, RuntimeKind::Podman, RuntimeKind::OpenSandbox] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: RuntimeKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn resource_limits_default() {
        let rl = ResourceLimits::default();
        assert!(rl.cpus.is_none());
        assert!(rl.memory_bytes.is_none());
        assert!(rl.pids_limit.is_none());
    }

    #[test]
    fn resource_limits_serde() {
        let rl = ResourceLimits {
            cpus: Some(2.5),
            memory_bytes: Some(1073741824),
            pids_limit: Some(100),
        };
        let json = serde_json::to_string(&rl).unwrap();
        let back: ResourceLimits = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cpus, Some(2.5));
        assert_eq!(back.memory_bytes, Some(1073741824));
        assert_eq!(back.pids_limit, Some(100));
    }

    #[test]
    fn container_config_serde_roundtrip() {
        let cfg = ContainerConfig {
            image: "rust:1.75".to_string(),
            name: Some("test-ctr".to_string()),
            env: vec![("KEY".to_string(), "VALUE".to_string())],
            volumes: vec![VolumeMount {
                host_path: "/src".to_string(),
                container_path: "/code".to_string(),
                read_only: true,
            }],
            resource_limits: ResourceLimits {
                cpus: Some(1.0),
                memory_bytes: None,
                pids_limit: None,
            },
            network_policy: NetworkPolicy::None,
            timeout_secs: 600,
            working_dir: Some("/code".to_string()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: ContainerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.image, "rust:1.75");
        assert_eq!(back.name, Some("test-ctr".to_string()));
        assert_eq!(back.env.len(), 1);
        assert_eq!(back.volumes.len(), 1);
        assert!(back.volumes[0].read_only);
        assert_eq!(back.timeout_secs, 600);
    }

    #[test]
    fn exec_result_serde() {
        let er = ExecResult {
            exit_code: 0,
            stdout: "hello\n".to_string(),
            stderr: "".to_string(),
        };
        let json = serde_json::to_string(&er).unwrap();
        let back: ExecResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.exit_code, 0);
        assert_eq!(back.stdout, "hello\n");
        assert!(back.stderr.is_empty());
    }

    #[test]
    fn exec_stream_event_variants_serde() {
        let events = vec![
            ExecStreamEvent::Stdout("out".to_string()),
            ExecStreamEvent::Stderr("err".to_string()),
            ExecStreamEvent::ExitCode(42),
            ExecStreamEvent::Error("fail".to_string()),
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let back: ExecStreamEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", back), format!("{:?}", event));
        }
    }

    #[test]
    fn container_metrics_serde() {
        let m = ContainerMetrics {
            cpu_usage_percent: 45.5,
            memory_used_bytes: 1024 * 1024 * 512,
            memory_limit_bytes: 1024 * 1024 * 1024,
            pids: 42,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: ContainerMetrics = serde_json::from_str(&json).unwrap();
        assert!((back.cpu_usage_percent - 45.5).abs() < f64::EPSILON);
        assert_eq!(back.pids, 42);
    }

    #[test]
    fn container_info_serde() {
        let info = ContainerInfo {
            id: "abc123".to_string(),
            name: "my-container".to_string(),
            image: "ubuntu:22.04".to_string(),
            status: "running".to_string(),
            created_at: "2024-01-01".to_string(),
            runtime: RuntimeKind::Docker,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ContainerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc123");
        assert_eq!(back.runtime, RuntimeKind::Docker);
    }

    #[test]
    fn volume_mount_serde() {
        let vm = VolumeMount {
            host_path: "/home/user/project".to_string(),
            container_path: "/workspace".to_string(),
            read_only: false,
        };
        let json = serde_json::to_string(&vm).unwrap();
        let back: VolumeMount = serde_json::from_str(&json).unwrap();
        assert_eq!(back.host_path, "/home/user/project");
        assert!(!back.read_only);
    }
}
