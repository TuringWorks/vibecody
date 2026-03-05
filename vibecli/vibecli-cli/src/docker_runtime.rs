#![allow(dead_code)]
//! Docker Container Runtime
//!
//! Implements [`ContainerRuntime`] using the `docker` CLI.

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::container_runtime::*;

/// Docker-backed container runtime.
pub struct DockerRuntime {
    binary: String,
}

impl DockerRuntime {
    pub fn new() -> Self {
        Self {
            binary: "docker".to_string(),
        }
    }

    /// Build the `docker run` arguments from a ContainerConfig.
    fn build_create_args(&self, config: &ContainerConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--label".to_string(),
            "vibecody=sandbox".to_string(),
        ];

        // Container name
        let name = config
            .name
            .clone()
            .unwrap_or_else(generate_container_name);
        args.push("--name".to_string());
        args.push(name);

        // Resource limits
        if let Some(cpus) = config.resource_limits.cpus {
            args.push("--cpus".to_string());
            args.push(format!("{cpus}"));
        }
        if let Some(mem) = config.resource_limits.memory_bytes {
            args.push("--memory".to_string());
            args.push(format!("{mem}"));
        }
        if let Some(pids) = config.resource_limits.pids_limit {
            args.push("--pids-limit".to_string());
            args.push(format!("{pids}"));
        }

        // Network policy
        match &config.network_policy {
            NetworkPolicy::None => {
                args.push("--network".to_string());
                args.push("none".to_string());
            }
            NetworkPolicy::Restricted { .. } => {
                // Start with bridge; iptables rules applied after creation
                args.push("--network".to_string());
                args.push("bridge".to_string());
            }
            NetworkPolicy::Full => {}
        }

        // Volumes
        for v in &config.volumes {
            args.push("-v".to_string());
            let ro = if v.read_only { ":ro" } else { ":rw" };
            args.push(format!("{}:{}{}", v.host_path, v.container_path, ro));
        }

        // Working directory
        if let Some(ref wd) = config.working_dir {
            args.push("-w".to_string());
            args.push(wd.clone());
        }

        // Environment variables
        for (k, v) in &config.env {
            args.push("-e".to_string());
            args.push(format!("{k}={v}"));
        }

        // Image + idle command
        args.push(config.image.clone());
        args.push("tail".to_string());
        args.push("-f".to_string());
        args.push("/dev/null".to_string());

        args
    }

    /// Apply iptables rules for restricted network policy inside the container.
    async fn apply_network_restrictions(
        &self,
        container_id: &str,
        allowed_domains: &[String],
    ) -> anyhow::Result<()> {
        // Build iptables script that blocks all egress except DNS + allowed domains
        let mut script = String::from(
            "iptables -F OUTPUT 2>/dev/null; \
             iptables -A OUTPUT -o lo -j ACCEPT; \
             iptables -A OUTPUT -p udp --dport 53 -j ACCEPT; \
             iptables -A OUTPUT -p tcp --dport 53 -j ACCEPT; \
             iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT; ",
        );

        for domain in allowed_domains {
            // Resolve domain to IPs and allow them
            script.push_str(&format!(
                "for ip in $(getent hosts {domain} 2>/dev/null | awk '{{print $1}}'); do \
                 iptables -A OUTPUT -d $ip -j ACCEPT; done; "
            ));
        }

        script.push_str("iptables -A OUTPUT -j DROP");

        let output = Command::new(&self.binary)
            .args(["exec", container_id, "sh", "-c", &script])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                "Network restriction setup failed (container may lack iptables): {stderr}"
            );
        }

        Ok(())
    }

    /// Run a docker command and collect stdout.
    async fn run_cmd(&self, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(&self.binary).args(args).output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("docker {} failed: {}", args[0], stderr.trim());
        }
    }

    /// Parse `docker ps --format json` output into ContainerInfo entries.
    fn parse_ps_json(&self, json_lines: &str) -> Vec<ContainerInfo> {
        json_lines
            .lines()
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                Some(ContainerInfo {
                    id: v["ID"].as_str().unwrap_or("").to_string(),
                    name: v["Names"].as_str().unwrap_or("").to_string(),
                    image: v["Image"].as_str().unwrap_or("").to_string(),
                    status: v["Status"].as_str().unwrap_or("").to_string(),
                    created_at: v["CreatedAt"].as_str().unwrap_or("").to_string(),
                    runtime: RuntimeKind::Docker,
                })
            })
            .collect()
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Docker
    }

    async fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .args(["version", "--format", "{{.Server.Version}}"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn version(&self) -> anyhow::Result<String> {
        self.run_cmd(&["version", "--format", "{{.Server.Version}}"])
            .await
    }

    async fn create(&self, config: &ContainerConfig) -> anyhow::Result<ContainerInfo> {
        let args = self.build_create_args(config);
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let output = Command::new(&self.binary)
            .args(&str_args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("docker run failed: {}", stderr.trim());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Apply network restrictions if needed
        if let NetworkPolicy::Restricted { ref allowed_domains } = config.network_policy {
            self.apply_network_restrictions(&container_id, allowed_domains)
                .await?;
        }

        self.inspect(&container_id).await
    }

    async fn stop(&self, id: &str) -> anyhow::Result<()> {
        self.run_cmd(&["stop", id]).await?;
        Ok(())
    }

    async fn remove(&self, id: &str) -> anyhow::Result<()> {
        self.run_cmd(&["rm", "-f", id]).await?;
        Ok(())
    }

    async fn pause(&self, id: &str) -> anyhow::Result<()> {
        self.run_cmd(&["pause", id]).await?;
        Ok(())
    }

    async fn resume(&self, id: &str) -> anyhow::Result<()> {
        self.run_cmd(&["unpause", id]).await?;
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<ContainerInfo>> {
        let output = self
            .run_cmd(&[
                "ps",
                "-a",
                "--filter",
                "label=vibecody=sandbox",
                "--format",
                "json",
            ])
            .await?;
        Ok(self.parse_ps_json(&output))
    }

    async fn inspect(&self, id: &str) -> anyhow::Result<ContainerInfo> {
        let output = self
            .run_cmd(&[
                "inspect",
                "--format",
                "{{.Id}}|{{.Name}}|{{.Config.Image}}|{{.State.Status}}|{{.Created}}",
                id,
            ])
            .await?;

        let parts: Vec<&str> = output.splitn(5, '|').collect();
        if parts.len() < 5 {
            anyhow::bail!("unexpected docker inspect output: {output}");
        }

        Ok(ContainerInfo {
            id: parts[0].to_string(),
            name: parts[1].trim_start_matches('/').to_string(),
            image: parts[2].to_string(),
            status: parts[3].to_string(),
            created_at: parts[4].to_string(),
            runtime: RuntimeKind::Docker,
        })
    }

    async fn exec(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
    ) -> anyhow::Result<ExecResult> {
        let mut args = vec!["exec"];
        if let Some(dir) = cwd {
            args.push("-w");
            args.push(dir);
        }
        args.push(id);
        args.push("sh");
        args.push("-c");
        args.push(command);

        let output = Command::new(&self.binary).args(&args).output().await?;

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn exec_stream(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
        tx: mpsc::Sender<ExecStreamEvent>,
    ) -> anyhow::Result<()> {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("exec");
        if let Some(dir) = cwd {
            cmd.arg("-w").arg(dir);
        }
        cmd.arg(id).arg("sh").arg("-c").arg(command);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let tx2 = tx.clone();

        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx.send(ExecStreamEvent::Stdout(line)).await;
                }
            }
        });

        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx2.send(ExecStreamEvent::Stderr(line)).await;
                }
            }
        });

        let _ = tokio::join!(stdout_task, stderr_task);

        let _status = child.wait().await?;

        Ok(())
    }

    async fn read_file(&self, id: &str, path: &str) -> anyhow::Result<String> {
        let result = self.exec(id, &format!("cat '{path}'"), None).await?;
        if result.exit_code != 0 {
            anyhow::bail!("read_file failed: {}", result.stderr);
        }
        Ok(result.stdout)
    }

    async fn write_file(&self, id: &str, path: &str, content: &str) -> anyhow::Result<()> {
        // Use docker exec with stdin pipe
        let mut child = Command::new(&self.binary)
            .args(["exec", "-i", id, "sh", "-c", &format!("cat > '{path}'")])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(content.as_bytes()).await?;
            drop(stdin);
        }

        let output = child.wait_with_output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("write_file failed: {}", stderr.trim());
        }
        Ok(())
    }

    async fn list_dir(&self, id: &str, path: &str) -> anyhow::Result<Vec<String>> {
        let result = self
            .exec(id, &format!("ls -1 '{path}'"), None)
            .await?;
        if result.exit_code != 0 {
            anyhow::bail!("list_dir failed: {}", result.stderr);
        }
        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    async fn logs(&self, id: &str, tail: Option<u32>) -> anyhow::Result<String> {
        let mut args = vec!["logs"];
        let tail_str;
        if let Some(n) = tail {
            args.push("--tail");
            tail_str = format!("{n}");
            args.push(&tail_str);
        }
        args.push(id);
        self.run_cmd(&args).await
    }

    async fn metrics(&self, id: &str) -> anyhow::Result<ContainerMetrics> {
        let output = self
            .run_cmd(&["stats", "--no-stream", "--format", "json", id])
            .await?;

        // Docker stats JSON has CPUPerc, MemUsage, PIDs
        let v: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|_| serde_json::json!({}));

        let cpu_str = v["CPUPerc"].as_str().unwrap_or("0%");
        let cpu = cpu_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);

        let pids_str = v["PIDs"].as_str().unwrap_or("0");
        let pids = pids_str.parse::<u32>().unwrap_or(0);

        // MemUsage format: "123.4MiB / 4GiB"
        let mem_str = v["MemUsage"].as_str().unwrap_or("0B / 0B");
        let mem_parts: Vec<&str> = mem_str.split('/').collect();
        let mem_used = parse_docker_mem_value(mem_parts.first().unwrap_or(&"0").trim());
        let mem_limit = parse_docker_mem_value(mem_parts.get(1).unwrap_or(&"0").trim());

        Ok(ContainerMetrics {
            cpu_usage_percent: cpu,
            memory_used_bytes: mem_used,
            memory_limit_bytes: mem_limit,
            pids,
        })
    }
}

/// Parse Docker memory values like "123.4MiB" or "4GiB" into bytes.
fn parse_docker_mem_value(s: &str) -> u64 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("GiB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1024.0 * 1024.0 * 1024.0) as u64
    } else if let Some(num) = s.strip_suffix("MiB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1024.0 * 1024.0) as u64
    } else if let Some(num) = s.strip_suffix("KiB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1024.0) as u64
    } else if let Some(num) = s.strip_suffix("B") {
        num.trim().parse().unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_create_args_basic() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "node:20".to_string(),
            name: Some("test-sb".to_string()),
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"test-sb".to_string()));
        assert!(args.contains(&"node:20".to_string()));
        assert!(args.contains(&"--label".to_string()));
        assert!(args.contains(&"vibecody=sandbox".to_string()));
    }

    #[test]
    fn build_create_args_resource_limits() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "ubuntu:22.04".to_string(),
            name: Some("rl-test".to_string()),
            resource_limits: ResourceLimits {
                cpus: Some(2.5),
                memory_bytes: Some(4294967296),
                pids_limit: Some(256),
            },
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"--cpus".to_string()));
        assert!(args.contains(&"2.5".to_string()));
        assert!(args.contains(&"--memory".to_string()));
        assert!(args.contains(&"4294967296".to_string()));
        assert!(args.contains(&"--pids-limit".to_string()));
        assert!(args.contains(&"256".to_string()));
    }

    #[test]
    fn build_create_args_network_none() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "alpine:3".to_string(),
            name: Some("net-test".to_string()),
            network_policy: NetworkPolicy::None,
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "none");
    }

    #[test]
    fn build_create_args_network_restricted() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "alpine:3".to_string(),
            name: Some("net-r".to_string()),
            network_policy: NetworkPolicy::Restricted {
                allowed_domains: vec!["github.com".to_string()],
            },
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "bridge");
    }

    #[test]
    fn build_create_args_volumes() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "ubuntu:22.04".to_string(),
            name: Some("vol-test".to_string()),
            volumes: vec![
                VolumeMount {
                    host_path: "/home/user/code".to_string(),
                    container_path: "/workspace".to_string(),
                    read_only: false,
                },
                VolumeMount {
                    host_path: "/etc/hosts".to_string(),
                    container_path: "/etc/hosts".to_string(),
                    read_only: true,
                },
            ],
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"/home/user/code:/workspace:rw".to_string()));
        assert!(args.contains(&"/etc/hosts:/etc/hosts:ro".to_string()));
    }

    #[test]
    fn build_create_args_env_vars() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "node:20".to_string(),
            name: Some("env-test".to_string()),
            env: vec![
                ("NODE_ENV".to_string(), "production".to_string()),
                ("PORT".to_string(), "3000".to_string()),
            ],
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"NODE_ENV=production".to_string()));
        assert!(args.contains(&"PORT=3000".to_string()));
    }

    #[test]
    fn build_create_args_working_dir() {
        let rt = DockerRuntime::new();
        let config = ContainerConfig {
            image: "ubuntu:22.04".to_string(),
            name: Some("wd-test".to_string()),
            working_dir: Some("/app".to_string()),
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        let w_idx = args.iter().position(|a| a == "-w").unwrap();
        assert_eq!(args[w_idx + 1], "/app");
    }

    #[test]
    fn parse_ps_json_entries() {
        let rt = DockerRuntime::new();
        let json = r#"{"ID":"abc123","Names":"vibecody-sb-1","Image":"ubuntu:22.04","Status":"Up 5 minutes","CreatedAt":"2024-01-01"}
{"ID":"def456","Names":"vibecody-sb-2","Image":"node:20","Status":"Exited","CreatedAt":"2024-01-02"}"#;
        let infos = rt.parse_ps_json(json);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, "abc123");
        assert_eq!(infos[0].name, "vibecody-sb-1");
        assert_eq!(infos[1].image, "node:20");
    }

    #[test]
    fn parse_docker_mem_gib() {
        assert_eq!(parse_docker_mem_value("4GiB"), 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_docker_mem_mib() {
        assert_eq!(parse_docker_mem_value("512MiB"), 512 * 1024 * 1024);
    }

    #[test]
    fn parse_docker_mem_kib() {
        assert_eq!(parse_docker_mem_value("1024KiB"), 1024 * 1024);
    }

    #[test]
    fn runtime_kind_is_docker() {
        let rt = DockerRuntime::new();
        assert_eq!(rt.kind(), RuntimeKind::Docker);
    }
}
