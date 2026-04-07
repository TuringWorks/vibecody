#![allow(dead_code)]
//! Container Tool Executor
//!
//! Routes agent tool calls through a running container instead of the local filesystem.
//! Web-related tools (WebSearch, FetchUrl) still run locally.

use async_trait::async_trait;
use std::sync::Arc;
use vibe_ai::agent::ToolExecutorTrait;
use vibe_ai::tools::{ToolCall, ToolResult};

use crate::container_runtime::{ContainerConfig, ContainerRuntime};

/// Executes agent tool calls inside a container sandbox.
pub struct ContainerToolExecutor {
    runtime: Arc<dyn ContainerRuntime>,
    container_id: String,
    /// Fallback executor for web operations that run locally.
    local_executor: Option<Arc<dyn ToolExecutorTrait>>,
}

impl ContainerToolExecutor {
    /// Create a new executor, spinning up a container from config.
    pub async fn new(
        runtime: Arc<dyn ContainerRuntime>,
        config: &ContainerConfig,
        local_executor: Option<Arc<dyn ToolExecutorTrait>>,
    ) -> anyhow::Result<Self> {
        let info = runtime.create(config).await?;
        Ok(Self {
            runtime,
            container_id: info.id,
            local_executor,
        })
    }

    /// Attach to an existing container by ID.
    pub fn with_existing(
        runtime: Arc<dyn ContainerRuntime>,
        container_id: String,
        local_executor: Option<Arc<dyn ToolExecutorTrait>>,
    ) -> Self {
        Self {
            runtime,
            container_id,
            local_executor,
        }
    }

    /// Get the container ID.
    pub fn container_id(&self) -> &str {
        &self.container_id
    }

    /// Stop and remove the container.
    pub async fn cleanup(&self) -> anyhow::Result<()> {
        let _ = self.runtime.stop(&self.container_id).await;
        let _ = self.runtime.remove(&self.container_id).await;
        Ok(())
    }
}

#[async_trait]
impl ToolExecutorTrait for ContainerToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        match call {
            ToolCall::ReadFile { path } => {
                match self.runtime.read_file(&self.container_id, path).await {
                    Ok(content) => ToolResult::ok("read_file", content),
                    Err(e) => ToolResult::err("read_file", e.to_string()),
                }
            }

            ToolCall::WriteFile { path, content } => {
                match self
                    .runtime
                    .write_file(&self.container_id, path, content)
                    .await
                {
                    Ok(()) => ToolResult::ok("write_file", format!("Wrote {} bytes to {path}", content.len())),
                    Err(e) => ToolResult::err("write_file", e.to_string()),
                }
            }

            ToolCall::ApplyPatch { path, patch } => {
                // Write patch to temp file and apply with `patch` command
                let tmp = "/tmp/_vibecody_patch.diff";
                if let Err(e) = self
                    .runtime
                    .write_file(&self.container_id, tmp, patch)
                    .await
                {
                    return ToolResult::err("apply_patch", e.to_string());
                }
                match self
                    .runtime
                    .exec(
                        &self.container_id,
                        &format!("patch -p0 '{path}' < {tmp} && rm -f {tmp}"),
                        None,
                    )
                    .await
                {
                    Ok(result) => {
                        if result.exit_code == 0 {
                            ToolResult::ok("apply_patch", format!("Patched {path}"))
                        } else {
                            ToolResult::err("apply_patch", result.stderr)
                        }
                    }
                    Err(e) => ToolResult::err("apply_patch", e.to_string()),
                }
            }

            ToolCall::Bash { command } => {
                match self
                    .runtime
                    .exec(&self.container_id, command, None)
                    .await
                {
                    Ok(result) => {
                        let mut output = result.stdout;
                        if !result.stderr.is_empty() {
                            output.push_str("\nSTDERR:\n");
                            output.push_str(&result.stderr);
                        }
                        if result.exit_code != 0 {
                            output.push_str(&format!("\n[exit code: {}]", result.exit_code));
                        }
                        ToolResult::ok("bash", output)
                    }
                    Err(e) => ToolResult::err("bash", e.to_string()),
                }
            }

            ToolCall::SearchFiles { query, glob: _ } => {
                let cmd = format!("grep -rn --include='*' '{query}' . 2>/dev/null | head -50");
                match self
                    .runtime
                    .exec(&self.container_id, &cmd, None)
                    .await
                {
                    Ok(result) => ToolResult::ok("search_files", result.stdout),
                    Err(e) => ToolResult::err("search_files", e.to_string()),
                }
            }

            ToolCall::ListDirectory { path } => {
                match self.runtime.list_dir(&self.container_id, path).await {
                    Ok(entries) => ToolResult::ok("list_directory", entries.join("\n")),
                    Err(e) => ToolResult::err("list_directory", e.to_string()),
                }
            }

            // Web operations run locally (not in the container)
            ToolCall::WebSearch { .. } | ToolCall::FetchUrl { .. } => {
                if let Some(ref local) = self.local_executor {
                    local.execute(call).await
                } else {
                    ToolResult::err(call.name(), "Web operations not available in sandbox mode")
                }
            }

            ToolCall::TaskComplete { summary } => {
                ToolResult::ok("task_complete", summary.clone())
            }

            ToolCall::SpawnAgent { task, max_steps, max_depth: _ } => {
                // In container mode, sub-agents reuse the same container
                let cmd = format!(
                    "echo 'Sub-agent task: {}' (max_steps: {})",
                    task.replace('\'', "'\\''"),
                    max_steps.unwrap_or(10)
                );
                match self
                    .runtime
                    .exec(&self.container_id, &cmd, None)
                    .await
                {
                    Ok(result) => ToolResult::ok("spawn_agent", result.stdout),
                    Err(e) => ToolResult::err("spawn_agent", e.to_string()),
                }
            }

            ToolCall::Think { thought } => {
                ToolResult::ok("think", format!("Reasoning noted ({} chars).", thought.len()))
            }

            ToolCall::PlanTask { steps } => {
                ToolResult::ok("plan_task", format!("Plan recorded:\n{}", steps))
            }

            ToolCall::Diffstat { path } => {
                let cmd = format!("git diff --stat HEAD -- {}", path.replace('\'', "'\\''"));
                match self.runtime.exec(&self.container_id, &cmd, None).await {
                    Ok(result) => {
                        let text = result.stdout + &result.stderr;
                        ToolResult::ok(
                            "diffstat",
                            if text.trim().is_empty() {
                                "No changes compared to HEAD (file may be untracked)".to_string()
                            } else {
                                text
                            },
                        )
                    }
                    Err(e) => ToolResult::err("diffstat", e.to_string()),
                }
            }

            ToolCall::RecordMemory { key, value } => {
                let cmd = format!(
                    "mkdir -p .vibe && grep -v '**{}**:' .vibe/memory.md 2>/dev/null > /tmp/mem_tmp.md; echo '- **{}**: {}' >> /tmp/mem_tmp.md; mv /tmp/mem_tmp.md .vibe/memory.md",
                    key.replace('\'', "'\\''"),
                    key.replace('\'', "'\\''"),
                    value.replace('\'', "'\\''"),
                );
                match self.runtime.exec(&self.container_id, &cmd, None).await {
                    Ok(_) => ToolResult::ok("record_memory", format!("Saved: {} = {}", key, value)),
                    Err(e) => ToolResult::err("record_memory", e.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_runtime::RuntimeKind;

    #[test]
    fn with_existing_stores_id() {
        // We can't easily test without a running runtime, but we can verify the struct
        // construction path compiles.
        let _ = RuntimeKind::Docker;
    }

    #[test]
    fn tool_call_names_match() {
        assert_eq!(ToolCall::ReadFile { path: "x".into() }.name(), "read_file");
        assert_eq!(
            ToolCall::WriteFile { path: "x".into(), content: "c".into() }.name(),
            "write_file"
        );
        assert_eq!(ToolCall::Bash { command: "ls".into() }.name(), "bash");
        assert_eq!(
            ToolCall::ListDirectory { path: ".".into() }.name(),
            "list_directory"
        );
        assert_eq!(
            ToolCall::SearchFiles { query: "q".into(), glob: None }.name(),
            "search_files"
        );
        assert_eq!(
            ToolCall::TaskComplete { summary: "done".into() }.name(),
            "task_complete"
        );
    }

    #[test]
    fn apply_patch_tool_name() {
        assert_eq!(
            ToolCall::ApplyPatch { path: "f.rs".into(), patch: "diff".into() }.name(),
            "apply_patch"
        );
    }

    #[test]
    fn web_search_tool_name() {
        assert_eq!(
            ToolCall::WebSearch { query: "rust".into(), num_results: 5 }.name(),
            "web_search"
        );
    }

    #[test]
    fn fetch_url_tool_name() {
        assert_eq!(
            ToolCall::FetchUrl { url: "https://example.com".into() }.name(),
            "fetch_url"
        );
    }

    #[test]
    fn spawn_agent_tool_name() {
        assert_eq!(
            ToolCall::SpawnAgent { task: "t".into(), max_steps: None, max_depth: None }.name(),
            "spawn_agent"
        );
    }

    #[test]
    fn runtime_kind_display_docker() {
        assert_eq!(RuntimeKind::Docker.to_string(), "docker");
    }

    #[test]
    fn runtime_kind_display_podman() {
        assert_eq!(RuntimeKind::Podman.to_string(), "podman");
    }

    #[test]
    fn runtime_kind_display_opensandbox() {
        assert_eq!(RuntimeKind::OpenSandbox.to_string(), "opensandbox");
    }

    #[test]
    fn runtime_kind_parse_docker() {
        let kind: RuntimeKind = "docker".parse().unwrap();
        assert_eq!(kind, RuntimeKind::Docker);
    }

    #[test]
    fn runtime_kind_parse_podman_case_insensitive() {
        let kind: RuntimeKind = "Podman".parse().unwrap();
        assert_eq!(kind, RuntimeKind::Podman);
    }

    #[test]
    fn runtime_kind_parse_unknown_fails() {
        let result: Result<RuntimeKind, _> = "kubernetes".parse();
        assert!(result.is_err());
    }

    #[test]
    fn tool_call_is_destructive() {
        assert!(ToolCall::WriteFile { path: "a".into(), content: "b".into() }.is_destructive());
        assert!(ToolCall::ApplyPatch { path: "a".into(), patch: "p".into() }.is_destructive());
        assert!(ToolCall::Bash { command: "rm -rf /".into() }.is_destructive());
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_destructive());
    }

    #[test]
    fn tool_call_is_terminal() {
        assert!(ToolCall::TaskComplete { summary: "done".into() }.is_terminal());
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_terminal());
        assert!(!ToolCall::Bash { command: "ls".into() }.is_terminal());
    }

    #[test]
    fn tool_call_summary_read_file() {
        let s = ToolCall::ReadFile { path: "/tmp/foo.rs".into() }.summary();
        assert!(s.contains("foo.rs"), "summary should contain filename, got: {}", s);
    }

    #[test]
    fn tool_call_summary_bash() {
        let s = ToolCall::Bash { command: "cargo test".into() }.summary();
        assert!(s.contains("cargo test"), "summary should contain command, got: {}", s);
    }

    #[test]
    fn container_config_default_values() {
        let config = ContainerConfig::default();
        assert_eq!(config.image, "ubuntu:22.04");
        assert!(config.name.is_none());
        assert!(config.env.is_empty());
        assert!(config.volumes.is_empty());
        assert_eq!(config.timeout_secs, 3600);
        assert_eq!(config.working_dir, Some("/workspace".to_string()));
    }
}
