//! `ToolExecutorTrait` implementation for the VibeUI Tauri backend.
//!
//! Executes agent tool calls using the local file system and shell,
//! without the sandbox facilities of the CLI (which relies on bwrap/sandbox-exec).

use async_trait::async_trait;
use std::path::PathBuf;
use vibe_ai::{ToolCall, ToolResult, ToolExecutorTrait};

const MAX_OUTPUT: usize = 8_000;

pub struct TauriToolExecutor {
    pub workspace_root: PathBuf,
}

impl TauriToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() { p } else { self.workspace_root.join(p) }
    }

    fn truncate(mut s: String) -> (String, bool) {
        if s.len() > MAX_OUTPUT {
            s.truncate(MAX_OUTPUT);
            s.push_str("\n…(truncated)");
            (s, true)
        } else {
            (s, false)
        }
    }

    async fn read_file(&self, path: &str) -> ToolResult {
        match std::fs::read_to_string(self.resolve(path)) {
            Ok(content) => {
                let (out, truncated) = Self::truncate(content);
                ToolResult { tool_name: "read_file".into(), output: out, success: true, truncated }
            }
            Err(e) => ToolResult::err("read_file", e.to_string()),
        }
    }

    async fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let p = self.resolve(path);
        if let Some(parent) = p.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::err("write_file", e.to_string());
            }
        }
        match std::fs::write(&p, content) {
            Ok(_) => ToolResult::ok(
                "write_file",
                format!("Wrote {} bytes to {}", content.len(), path),
            ),
            Err(e) => ToolResult::err("write_file", e.to_string()),
        }
    }

    async fn run_bash(&self, command: &str) -> ToolResult {
        use std::process::Command;
        match Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace_root)
            .output()
        {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                let mut raw = format!("exit: {}\n", o.status.code().unwrap_or(-1));
                if !stdout.is_empty() { raw.push_str("stdout:\n"); raw.push_str(&stdout); }
                if !stderr.is_empty() { raw.push_str("stderr:\n"); raw.push_str(&stderr); }
                let success = o.status.success();
                let (out, truncated) = Self::truncate(raw);
                ToolResult { tool_name: "bash".into(), output: out, success, truncated }
            }
            Err(e) => ToolResult::err("bash", e.to_string()),
        }
    }

    async fn search_files(&self, query: &str, glob: Option<&str>) -> ToolResult {
        match vibe_core::search::search_files(&self.workspace_root, query, false) {
            Ok(results) => {
                let iter = results.iter().filter(|r| {
                    glob.is_none_or(|g| r.path.contains(g))
                });
                let lines: Vec<String> = iter.take(30).map(|r| {
                    format!("{}:{}: {}", r.path, r.line_number, r.line_content.trim())
                }).collect();
                ToolResult::ok(
                    "search_files",
                    if lines.is_empty() { "No results.".into() } else { lines.join("\n") },
                )
            }
            Err(e) => ToolResult::err("search_files", e.to_string()),
        }
    }

    async fn web_search(&self, query: &str) -> ToolResult {
        let encoded = query.replace(' ', "+");
        let url = format!("https://lite.duckduckgo.com/lite/?q={}", encoded);
        match crate::commands::fetch_and_strip(&url).await {
            Ok(text) => {
                // Extract result lines (skip navigation chrome)
                let lines: Vec<&str> = text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .take(30)
                    .collect();
                ToolResult::ok("web_search", lines.join("\n"))
            }
            Err(e) => ToolResult::err("web_search", e),
        }
    }

    async fn fetch_url(&self, url: &str) -> ToolResult {
        match crate::commands::fetch_and_strip(url).await {
            Ok(text) => ToolResult::ok("fetch_url", format!("=== {} ===\n{}", url, text)),
            Err(e)   => ToolResult::err("fetch_url", e),
        }
    }

    async fn list_dir(&self, path: &str) -> ToolResult {
        let p = self.resolve(path);
        match std::fs::read_dir(&p) {
            Ok(entries) => {
                let mut items: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let name = e.file_name().to_string_lossy().to_string();
                        if is_dir { format!("{}/", name) } else { name }
                    })
                    .collect();
                items.sort();
                ToolResult::ok("list_directory", items.join("\n"))
            }
            Err(e) => ToolResult::err("list_directory", e.to_string()),
        }
    }
}

/// Convenience method callable without the trait in scope (used by approval command).
impl TauriToolExecutor {
    pub async fn execute_call(&self, call: &ToolCall) -> ToolResult {
        match call {
            ToolCall::ReadFile { path }          => self.read_file(path).await,
            ToolCall::WriteFile { path, content } => self.write_file(path, content).await,
            ToolCall::ApplyPatch { .. }           => ToolResult::err(
                "apply_patch",
                "apply_patch is not supported in VibeUI — use write_file instead.",
            ),
            ToolCall::Bash { command }            => self.run_bash(command).await,
            ToolCall::SearchFiles { query, glob } => self.search_files(query, glob.as_deref()).await,
            ToolCall::ListDirectory { path }      => self.list_dir(path).await,
            ToolCall::WebSearch { query, .. } => self.web_search(query).await,
            ToolCall::FetchUrl { url }         => self.fetch_url(url).await,
            ToolCall::TaskComplete { summary } => ToolResult::ok("task_complete", summary.clone()),
            ToolCall::SpawnAgent { .. }        => ToolResult::err(
                "spawn_agent",
                "spawn_agent is not supported in VibeUI — use the CLI for sub-agent spawning.",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate ─────────────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        let s = "hello".to_string();
        let (out, truncated) = TauriToolExecutor::truncate(s);
        assert_eq!(out, "hello");
        assert!(!truncated);
    }

    #[test]
    fn truncate_exact_limit_unchanged() {
        let s = "a".repeat(MAX_OUTPUT);
        let (out, truncated) = TauriToolExecutor::truncate(s);
        assert_eq!(out.len(), MAX_OUTPUT);
        assert!(!truncated);
    }

    #[test]
    fn truncate_over_limit() {
        let s = "a".repeat(MAX_OUTPUT + 100);
        let (out, truncated) = TauriToolExecutor::truncate(s);
        assert!(truncated);
        assert!(out.ends_with("…(truncated)"));
        // Length is MAX_OUTPUT + the truncation marker
        assert!(out.len() < MAX_OUTPUT + 100);
    }

    #[test]
    fn truncate_empty_string() {
        let (out, truncated) = TauriToolExecutor::truncate(String::new());
        assert_eq!(out, "");
        assert!(!truncated);
    }

    // ── resolve ──────────────────────────────────────────────────────────────

    #[test]
    fn resolve_absolute_path_returned_as_is() {
        let exec = TauriToolExecutor::new(PathBuf::from("/workspace"));
        let result = exec.resolve("/etc/passwd");
        assert_eq!(result, PathBuf::from("/etc/passwd"));
    }

    #[test]
    fn resolve_relative_path_joined_to_workspace() {
        let exec = TauriToolExecutor::new(PathBuf::from("/workspace"));
        let result = exec.resolve("src/main.rs");
        assert_eq!(result, PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn resolve_dot_path() {
        let exec = TauriToolExecutor::new(PathBuf::from("/workspace"));
        let result = exec.resolve(".");
        assert_eq!(result, PathBuf::from("/workspace/."));
    }

    #[test]
    fn resolve_empty_path() {
        let exec = TauriToolExecutor::new(PathBuf::from("/workspace"));
        let result = exec.resolve("");
        assert_eq!(result, PathBuf::from("/workspace/"));
    }

    // ── execute_call routing ─────────────────────────────────────────────────

    #[tokio::test]
    async fn execute_call_apply_patch_returns_error() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let call = ToolCall::ApplyPatch { path: "test.rs".into(), patch: "test".into() };
        let result = exec.execute_call(&call).await;
        assert!(!result.success);
        assert!(result.output.contains("not supported"));
    }

    #[tokio::test]
    async fn execute_call_spawn_agent_returns_error() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let call = ToolCall::SpawnAgent {
            task: "test".into(),
            max_steps: None,
            max_depth: None,
        };
        let result = exec.execute_call(&call).await;
        assert!(!result.success);
        assert!(result.output.contains("not supported"));
    }

    #[tokio::test]
    async fn execute_call_task_complete() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let call = ToolCall::TaskComplete { summary: "done".into() };
        let result = exec.execute_call(&call).await;
        assert!(result.success);
        assert_eq!(result.output, "done");
    }

    #[tokio::test]
    async fn execute_call_read_file_missing() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let call = ToolCall::ReadFile { path: "/tmp/nonexistent_vibeui_test_file_xyz".into() };
        let result = exec.execute_call(&call).await;
        assert!(!result.success);
    }
}

#[async_trait]
impl ToolExecutorTrait for TauriToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        match call {
            ToolCall::ReadFile { path }           => self.read_file(path).await,
            ToolCall::WriteFile { path, content } => self.write_file(path, content).await,
            ToolCall::ApplyPatch { .. }           => {
                // ApplyPatch requires a unified-diff engine; instruct the agent to use write_file.
                ToolResult::err("apply_patch", "apply_patch is not supported in VibeUI — use write_file with the complete file contents instead.")
            }
            ToolCall::Bash { command }            => self.run_bash(command).await,
            ToolCall::SearchFiles { query, glob } => self.search_files(query, glob.as_deref()).await,
            ToolCall::ListDirectory { path }      => self.list_dir(path).await,
            ToolCall::WebSearch { query, .. }     => self.web_search(query).await,
            ToolCall::FetchUrl { url }            => self.fetch_url(url).await,
            ToolCall::TaskComplete { summary }    => ToolResult::ok("task_complete", summary.clone()),
            ToolCall::SpawnAgent { .. }           => ToolResult::err(
                "spawn_agent",
                "spawn_agent is not supported in VibeUI — use the CLI for sub-agent spawning.",
            ),
        }
    }
}
