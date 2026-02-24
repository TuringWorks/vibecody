//! Executes agent tool calls against the local filesystem.

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use vibe_ai::agent::ToolExecutorTrait;
use vibe_ai::tools::{ToolCall, ToolResult};
use std::path::Path as StdPath;
use vibe_core::executor::CommandExecutor;
use vibe_core::search::search_files;

#[derive(Clone)]
pub struct ToolExecutor {
    pub workspace_root: PathBuf,
    pub sandbox: bool,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf, sandbox: bool) -> Self {
        Self { workspace_root, sandbox }
    }
}

#[async_trait]
impl ToolExecutorTrait for ToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        match call {
            ToolCall::ReadFile { path } => self.read_file(path).await,
            ToolCall::WriteFile { path, content } => self.write_file(path, content).await,
            ToolCall::ApplyPatch { path, patch } => self.apply_patch(path, patch).await,
            ToolCall::Bash { command } => self.run_bash(command).await,
            ToolCall::SearchFiles { query, glob } => self.search(query, glob.as_deref()).await,
            ToolCall::ListDirectory { path } => self.list_dir(path).await,
            ToolCall::TaskComplete { summary } => {
                ToolResult::ok("task_complete", format!("Task complete: {}", summary))
            }
        }
    }
}

impl ToolExecutor {
    async fn read_file(&self, path: &str) -> ToolResult {
        let resolved = self.resolve(path);
        match std::fs::read_to_string(&resolved) {
            Ok(content) => ToolResult::ok("read_file", content),
            Err(e) => ToolResult::err("read_file", format!("Cannot read {}: {}", resolved.display(), e)),
        }
    }

    async fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let resolved = self.resolve(path);
        if let Some(parent) = resolved.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::err("write_file", format!("Cannot create directories: {}", e));
            }
        }
        match std::fs::write(&resolved, content) {
            Ok(_) => ToolResult::ok(
                "write_file",
                format!("Written {} bytes to {}", content.len(), resolved.display()),
            ),
            Err(e) => ToolResult::err("write_file", format!("Cannot write {}: {}", resolved.display(), e)),
        }
    }

    async fn apply_patch(&self, path: &str, patch: &str) -> ToolResult {
        let resolved = self.resolve(path);
        let original = match std::fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult::err(
                    "apply_patch",
                    format!("Cannot read {}: {}", resolved.display(), e),
                )
            }
        };
        let patched = match apply_unified_patch(&original, patch) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("apply_patch", format!("Patch failed: {}", e)),
        };
        match std::fs::write(&resolved, &patched) {
            Ok(_) => ToolResult::ok("apply_patch", format!("Patch applied to {}", resolved.display())),
            Err(e) => ToolResult::err("apply_patch", format!("Cannot write patched file: {}", e)),
        }
    }

    async fn run_bash(&self, command: &str) -> ToolResult {
        let cwd = &self.workspace_root;
        let output = if self.sandbox {
            CommandExecutor::execute_sandboxed(command, cwd, cwd)
        } else {
            CommandExecutor::execute_in(command, cwd)
        };

        match output {
            Ok(out) => {
                let text = CommandExecutor::output_to_string(&out);
                if out.status.success() {
                    ToolResult::ok("bash", text)
                } else {
                    let code = out.status.code().unwrap_or(-1);
                    ToolResult {
                        tool_name: "bash".into(),
                        output: format!("[exit {}]\n{}", code, text),
                        success: false,
                        truncated: false,
                    }
                }
            }
            Err(e) => ToolResult::err("bash", format!("Execution failed: {}", e)),
        }
    }

    async fn search(&self, query: &str, glob: Option<&str>) -> ToolResult {
        let root = &self.workspace_root;
        match search_files(root, query, false) {
            Ok(results) => {
                if results.is_empty() {
                    return ToolResult::ok("search_files", "No matches found.");
                }
                let mut output = String::new();
                for r in results.iter().take(50) {
                    if let Some(pattern) = glob {
                        let file_name = StdPath::new(&r.path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        if !glob_match(pattern, file_name) {
                            continue;
                        }
                    }
                    output.push_str(&format!(
                        "{}:{}: {}\n",
                        r.path,
                        r.line_number,
                        r.line_content.trim()
                    ));
                }
                if output.is_empty() {
                    ToolResult::ok("search_files", "No matches after glob filter.")
                } else {
                    ToolResult::ok("search_files", output)
                }
            }
            Err(e) => ToolResult::err("search_files", format!("Search failed: {}", e)),
        }
    }

    async fn list_dir(&self, path: &str) -> ToolResult {
        let resolved = self.resolve(path);
        match std::fs::read_dir(&resolved) {
            Ok(entries) => {
                let mut lines = Vec::new();
                for entry in entries.flatten() {
                    let meta = entry.metadata().ok();
                    let is_dir = meta.map(|m| m.is_dir()).unwrap_or(false);
                    let name = entry.file_name().to_string_lossy().to_string();
                    lines.push(if is_dir { format!("{}/", name) } else { name });
                }
                lines.sort();
                ToolResult::ok("list_directory", lines.join("\n"))
            }
            Err(e) => ToolResult::err(
                "list_directory",
                format!("Cannot list {}: {}", resolved.display(), e),
            ),
        }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.workspace_root.join(p)
        }
    }
}

/// Apply a unified diff patch string to source text in-process.
fn apply_unified_patch(original: &str, patch: &str) -> Result<String> {
    let orig_lines: Vec<&str> = original.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut orig_idx = 0usize;

    for chunk in patch.split("\n@@") {
        if chunk.trim().is_empty() || !chunk.contains("@@") && result.is_empty() {
            continue;
        }
        let hunk_str = if chunk.starts_with("@@") {
            chunk.to_string()
        } else {
            format!("@@{}", chunk)
        };

        let header_end = hunk_str.find('\n').unwrap_or(hunk_str.len());
        let header = &hunk_str[..header_end];
        let old_start = parse_hunk_start(header, '-')?;
        let old_start_0 = old_start.saturating_sub(1);

        while orig_idx < old_start_0 && orig_idx < orig_lines.len() {
            result.push(orig_lines[orig_idx].to_string());
            orig_idx += 1;
        }

        for line in hunk_str[header_end..].lines().skip(1) {
            if line.starts_with('-') {
                orig_idx += 1;
            } else if line.starts_with('+') {
                result.push(line[1..].to_string());
            } else if line.starts_with(' ') || line.is_empty() {
                if orig_idx < orig_lines.len() {
                    result.push(orig_lines[orig_idx].to_string());
                }
                orig_idx += 1;
            }
        }
    }

    while orig_idx < orig_lines.len() {
        result.push(orig_lines[orig_idx].to_string());
        orig_idx += 1;
    }

    Ok(result.join("\n"))
}

fn parse_hunk_start(header: &str, sign: char) -> Result<usize> {
    for part in header.split_whitespace() {
        if part.starts_with(sign) {
            let nums = part.trim_start_matches(sign);
            let start = nums.split(',').next().unwrap_or("1");
            return Ok(start.parse::<usize>().unwrap_or(1));
        }
    }
    Ok(1)
}

/// Very simple glob matching: only `*` wildcard supported.
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(ext) = pattern.strip_prefix("*.") {
        return name.ends_with(&format!(".{}", ext));
    }
    name == pattern
}
