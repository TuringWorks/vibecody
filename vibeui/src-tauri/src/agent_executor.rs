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
                    glob.map_or(true, |g| r.path.contains(g))
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
            ToolCall::TaskComplete { summary }    => ToolResult::ok("task_complete", summary.clone()),
        }
    }
}

#[async_trait]
impl ToolExecutorTrait for TauriToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        match call {
            ToolCall::ReadFile { path }         => self.read_file(path).await,
            ToolCall::WriteFile { path, content } => self.write_file(path, content).await,
            ToolCall::ApplyPatch { .. }         => {
                // ApplyPatch requires a unified-diff engine; instruct the agent to use write_file.
                ToolResult::err("apply_patch", "apply_patch is not supported in VibeUI — use write_file with the complete file contents instead.")
            }
            ToolCall::Bash { command }          => self.run_bash(command).await,
            ToolCall::SearchFiles { query, glob } => self.search_files(query, glob.as_deref()).await,
            ToolCall::ListDirectory { path }    => self.list_dir(path).await,
            ToolCall::TaskComplete { summary }  => ToolResult::ok("task_complete", summary.clone()),
        }
    }
}
