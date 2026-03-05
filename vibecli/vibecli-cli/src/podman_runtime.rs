#![allow(dead_code)]
//! Podman Container Runtime
//!
//! Implements [`ContainerRuntime`] using the `podman` CLI.
//! Provides full feature parity with the Docker runtime plus Podman-specific
//! features like native rootless support and `podman machine` awareness on macOS.

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::container_runtime::*;

/// Podman-backed container runtime.
pub struct PodmanRuntime {
    binary: String,
}

impl PodmanRuntime {
    pub fn new() -> Self {
        Self {
            binary: "podman".to_string(),
        }
    }

    /// Check if running in rootless mode.
    #[allow(dead_code)]
    pub async fn is_rootless(&self) -> bool {
        Command::new(&self.binary)
            .args(["info", "--format", "{{.Host.Security.Rootless}}"])
            .output()
            .await
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout).trim() == "true"
            })
            .unwrap_or(false)
    }

    /// Check if a Podman machine is running (macOS/Windows VM).
    #[allow(dead_code)]
    pub async fn is_machine_running(&self) -> bool {
        Command::new(&self.binary)
            .args(["machine", "info"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Build `podman run` arguments from a ContainerConfig.
    fn build_create_args(&self, config: &ContainerConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--label".to_string(),
            "vibecody=sandbox".to_string(),
        ];

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
                // Podman uses slirp4netns or pasta by default; bridge for restriction
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

    /// Apply iptables rules for restricted network inside the container.
    async fn apply_network_restrictions(
        &self,
        container_id: &str,
        allowed_domains: &[String],
    ) -> anyhow::Result<()> {
        let mut script = String::from(
            "iptables -F OUTPUT 2>/dev/null; \
             iptables -A OUTPUT -o lo -j ACCEPT; \
             iptables -A OUTPUT -p udp --dport 53 -j ACCEPT; \
             iptables -A OUTPUT -p tcp --dport 53 -j ACCEPT; \
             iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT; ",
        );

        for domain in allowed_domains {
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

    /// Run a podman command and collect stdout.
    async fn run_cmd(&self, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(&self.binary).args(args).output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("podman {} failed: {}", args[0], stderr.trim());
        }
    }

    /// Parse `podman ps --format json` output.
    fn parse_ps_json(&self, json_str: &str) -> Vec<ContainerInfo> {
        // Podman `--format json` returns a JSON array (unlike Docker's JSONL)
        let arr: Vec<serde_json::Value> =
            serde_json::from_str(json_str).unwrap_or_default();

        arr.into_iter()
            .map(|v| ContainerInfo {
                id: v["Id"]
                    .as_str()
                    .or_else(|| v["ID"].as_str())
                    .unwrap_or("")
                    .to_string(),
                name: v["Names"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|n| n.as_str())
                    .or_else(|| v["Names"].as_str())
                    .unwrap_or("")
                    .to_string(),
                image: v["Image"].as_str().unwrap_or("").to_string(),
                status: v["Status"]
                    .as_str()
                    .or_else(|| v["State"].as_str())
                    .unwrap_or("")
                    .to_string(),
                created_at: v["CreatedAt"]
                    .as_str()
                    .or_else(|| v["Created"].as_str())
                    .unwrap_or("")
                    .to_string(),
                runtime: RuntimeKind::Podman,
            })
            .collect()
    }
}

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Podman
    }

    async fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .args(["version", "--format", "{{.Version}}"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn version(&self) -> anyhow::Result<String> {
        self.run_cmd(&["version", "--format", "{{.Version}}"])
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
            anyhow::bail!("podman run failed: {}", stderr.trim());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

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
            anyhow::bail!("unexpected podman inspect output: {output}");
        }

        Ok(ContainerInfo {
            id: parts[0].to_string(),
            name: parts[1].trim_start_matches('/').to_string(),
            image: parts[2].to_string(),
            status: parts[3].to_string(),
            created_at: parts[4].to_string(),
            runtime: RuntimeKind::Podman,
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
        let _ = child.wait().await?;

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
        let result = self.exec(id, &format!("ls -1 '{path}'"), None).await?;
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
        // Podman stats --format json returns a JSON array
        let output = self
            .run_cmd(&["stats", "--no-stream", "--format", "json", id])
            .await?;

        let arr: Vec<serde_json::Value> =
            serde_json::from_str(&output).unwrap_or_default();
        let v = arr.first().cloned().unwrap_or_else(|| serde_json::json!({}));

        let cpu = v["cpu_percent"]
            .as_f64()
            .or_else(|| {
                v["CPUPerc"]
                    .as_str()
                    .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
            })
            .unwrap_or(0.0);

        let mem_used = v["mem_usage"]
            .as_u64()
            .or_else(|| {
                v["MemUsage"]
                    .as_str()
                    .map(|s| parse_podman_mem_value(s.split('/').next().unwrap_or("0").trim()))
            })
            .unwrap_or(0);

        let mem_limit = v["mem_limit"]
            .as_u64()
            .or_else(|| {
                v["MemUsage"]
                    .as_str()
                    .and_then(|s| s.split('/').nth(1))
                    .map(|s| parse_podman_mem_value(s.trim()))
            })
            .unwrap_or(0);

        let pids = v["pids"]
            .as_u64()
            .or_else(|| v["PIDs"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0) as u32;

        Ok(ContainerMetrics {
            cpu_usage_percent: cpu,
            memory_used_bytes: mem_used,
            memory_limit_bytes: mem_limit,
            pids,
        })
    }
}

/// Parse Podman memory values like "123.4MB" or "4GB" into bytes.
fn parse_podman_mem_value(s: &str) -> u64 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("GB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1_000_000_000.0) as u64
    } else if let Some(num) = s.strip_suffix("GiB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1024.0 * 1024.0 * 1024.0) as u64
    } else if let Some(num) = s.strip_suffix("MB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1_000_000.0) as u64
    } else if let Some(num) = s.strip_suffix("MiB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1024.0 * 1024.0) as u64
    } else if let Some(num) = s.strip_suffix("KB") {
        let n: f64 = num.trim().parse().unwrap_or(0.0);
        (n * 1000.0) as u64
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
        let rt = PodmanRuntime::new();
        let config = ContainerConfig {
            image: "python:3.12".to_string(),
            name: Some("pm-test".to_string()),
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"pm-test".to_string()));
        assert!(args.contains(&"python:3.12".to_string()));
        assert!(args.contains(&"--label".to_string()));
        assert!(args.contains(&"vibecody=sandbox".to_string()));
    }

    #[test]
    fn build_create_args_resource_limits() {
        let rt = PodmanRuntime::new();
        let config = ContainerConfig {
            image: "ubuntu:22.04".to_string(),
            name: Some("pm-rl".to_string()),
            resource_limits: ResourceLimits {
                cpus: Some(4.0),
                memory_bytes: Some(8589934592),
                pids_limit: Some(512),
            },
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        assert!(args.contains(&"--cpus".to_string()));
        assert!(args.contains(&"4".to_string()));
        assert!(args.contains(&"--memory".to_string()));
        assert!(args.contains(&"--pids-limit".to_string()));
    }

    #[test]
    fn build_create_args_network_none() {
        let rt = PodmanRuntime::new();
        let config = ContainerConfig {
            image: "alpine:3".to_string(),
            name: Some("pm-net".to_string()),
            network_policy: NetworkPolicy::None,
            ..Default::default()
        };
        let args = rt.build_create_args(&config);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "none");
    }

    #[test]
    fn parse_ps_json_array() {
        let rt = PodmanRuntime::new();
        let json = r#"[
            {"Id":"abc123","Names":["vibecody-sb-1"],"Image":"ubuntu:22.04","State":"running","Created":"2024-01-01"},
            {"Id":"def456","Names":["vibecody-sb-2"],"Image":"node:20","State":"exited","Created":"2024-01-02"}
        ]"#;
        let infos = rt.parse_ps_json(json);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, "abc123");
        assert_eq!(infos[0].name, "vibecody-sb-1");
        assert_eq!(infos[0].runtime, RuntimeKind::Podman);
        assert_eq!(infos[1].image, "node:20");
    }

    #[test]
    fn parse_podman_mem_gb() {
        assert_eq!(parse_podman_mem_value("4GB"), 4_000_000_000);
    }

    #[test]
    fn parse_podman_mem_mb() {
        assert_eq!(parse_podman_mem_value("512MB"), 512_000_000);
    }

    #[test]
    fn parse_podman_mem_gib() {
        assert_eq!(parse_podman_mem_value("4GiB"), 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn runtime_kind_is_podman() {
        let rt = PodmanRuntime::new();
        assert_eq!(rt.kind(), RuntimeKind::Podman);
    }
}
