//! `ToolExecutorTrait` implementation for the VibeUI Tauri backend.
//!
//! Executes agent tool calls using the local file system and shell,
//! without the sandbox facilities of the CLI (which relies on bwrap/sandbox-exec).

use async_trait::async_trait;
use std::path::PathBuf;
use vibe_ai::{ToolCall, ToolResult, ToolExecutorTrait};

const MAX_OUTPUT: usize = 8_000;
/// Maximum wall-clock time for a single bash command (seconds).
const BASH_TIMEOUT_SECS: u64 = 120;

/// Validate a URL against SSRF attacks. Blocks internal IPs, metadata endpoints,
/// and non-HTTP schemes.
fn validate_url_for_ssrf(url: &str) -> Result<(), String> {
    let lower = url.to_lowercase();

    // Only allow http:// and https://
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!("URL scheme not allowed: only http/https permitted (got '{}')", url));
    }

    // Extract hostname
    let after_scheme = if lower.starts_with("https://") { &lower[8..] } else { &lower[7..] };
    let host = after_scheme.split('/').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or(""); // strip port

    // Block loopback
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
        return Err("SSRF blocked: localhost/loopback addresses not allowed".to_string());
    }

    // Block cloud metadata endpoints
    if host == "169.254.169.254" || host == "metadata.google.internal" {
        return Err("SSRF blocked: cloud metadata endpoint not allowed".to_string());
    }

    // Block private IP ranges (RFC 1918 + link-local)
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        if ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() {
            return Err(format!("SSRF blocked: private/internal IP {} not allowed", ip));
        }
        // Also block 169.254.x.x explicitly
        if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
            return Err(format!("SSRF blocked: link-local IP {} not allowed", ip));
        }
    }

    Ok(())
}

pub struct TauriToolExecutor {
    pub workspace_root: PathBuf,
}

impl TauriToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Resolve a path safely within the workspace boundary.
    /// Rejects absolute paths and `..` traversals that escape the workspace.
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let p = PathBuf::from(path);
        let resolved = if p.is_absolute() { p } else { self.workspace_root.join(p) };

        // Canonicalize to resolve symlinks and `..` components.
        // If the path doesn't exist yet (e.g. new file), canonicalize the parent.
        let canonical = if resolved.exists() {
            resolved.canonicalize().map_err(|e| format!("Path error: {}", e))?
        } else if let Some(parent) = resolved.parent() {
            if parent.exists() {
                let canon_parent = parent.canonicalize().map_err(|e| format!("Path error: {}", e))?;
                canon_parent.join(resolved.file_name().unwrap_or_default())
            } else {
                resolved.clone()
            }
        } else {
            resolved.clone()
        };

        // Ensure the resolved path is within the workspace
        let workspace_canonical = self.workspace_root.canonicalize()
            .unwrap_or_else(|_| self.workspace_root.clone());
        if !canonical.starts_with(&workspace_canonical) {
            return Err(format!(
                "Path traversal blocked: '{}' resolves outside workspace '{}'",
                path, workspace_canonical.display()
            ));
        }

        Ok(canonical)
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
        let resolved = match self.resolve(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("read_file", e),
        };
        match std::fs::read_to_string(resolved) {
            Ok(content) => {
                let (out, truncated) = Self::truncate(content);
                ToolResult { tool_name: "read_file".into(), output: out, success: true, truncated }
            }
            Err(e) => ToolResult::err("read_file", e.to_string()),
        }
    }

    async fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let p = match self.resolve(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("write_file", e),
        };
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

    /// Check if a shell command is blocked (destructive, exfiltration, etc.).
    fn is_blocked_command(command: &str) -> Option<&'static str> {
        let lower = command.to_lowercase();
        let blocked = [
            ("rm -rf /", "destructive: rm -rf /"),
            ("rm -rf /*", "destructive: rm -rf /*"),
            ("mkfs", "destructive: mkfs"),
            ("dd if=", "destructive: dd"),
            (":(){ :|:& };:", "fork bomb"),
            ("fork bomb", "fork bomb"),
            ("poweroff", "system shutdown"),
            ("reboot", "system reboot"),
            ("halt", "system halt"),
            ("shutdown", "system shutdown"),
            ("chmod -r 777 /", "destructive permissions"),
            ("curl -d", "potential data exfiltration"),
            ("wget --post-data", "potential data exfiltration"),
            ("/dev/tcp/", "reverse shell"),
            ("base64 -d|sh", "encoded execution"),
            ("base64 -d | sh", "encoded execution"),
            ("> /dev/sd", "disk overwrite"),
            ("iptables", "firewall manipulation"),
            ("crontab", "persistence mechanism"),
        ];
        for (pattern, reason) in &blocked {
            if lower.contains(pattern) {
                return Some(reason);
            }
        }
        None
    }

    async fn run_bash(&self, command: &str) -> ToolResult {
        // Security: block dangerous commands
        if let Some(reason) = Self::is_blocked_command(command) {
            return ToolResult::err("bash", format!("Command blocked: {}", reason));
        }

        // Run with timeout to prevent DoS
        use tokio::process::Command;
        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => return ToolResult::err("bash", e.to_string()),
        };

        let timeout = tokio::time::Duration::from_secs(BASH_TIMEOUT_SECS);
        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(o)) => {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
                let mut raw = format!("exit: {}\n", o.status.code().unwrap_or(-1));
                if !stdout.is_empty() { raw.push_str("stdout:\n"); raw.push_str(&stdout); }
                if !stderr.is_empty() { raw.push_str("stderr:\n"); raw.push_str(&stderr); }
                let success = o.status.success();
                let (out, truncated) = Self::truncate(raw);
                ToolResult { tool_name: "bash".into(), output: out, success, truncated }
            }
            Ok(Err(e)) => ToolResult::err("bash", e.to_string()),
            Err(_) => {
                ToolResult::err("bash", format!("Command timed out after {}s", BASH_TIMEOUT_SECS))
            }
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
        // SSRF protection: block internal/metadata URLs
        if let Err(reason) = validate_url_for_ssrf(url) {
            return ToolResult::err("fetch_url", reason);
        }
        match crate::commands::fetch_and_strip(url).await {
            Ok(text) => ToolResult::ok("fetch_url", format!("=== {} ===\n{}", url, text)),
            Err(e)   => ToolResult::err("fetch_url", e),
        }
    }

    async fn list_dir(&self, path: &str) -> ToolResult {
        let p = match self.resolve(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("list_directory", e),
        };
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
            ToolCall::Think { thought } => {
                ToolResult::ok("think", format!("Reasoning noted ({} chars).", thought.len()))
            }
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

    // ── resolve (security-hardened) ─────────────────────────────────────────

    #[test]
    fn resolve_absolute_path_outside_workspace_blocked() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let result = exec.resolve("/etc/passwd");
        assert!(result.is_err(), "absolute path outside workspace must be blocked");
        assert!(result.unwrap_err().contains("traversal blocked"));
    }

    #[test]
    fn resolve_relative_path_within_workspace_ok() {
        // Use /tmp as workspace so canonicalization works
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let result = exec.resolve("test_file.rs");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("/"));
    }

    #[test]
    fn resolve_dot_dot_traversal_blocked() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let result = exec.resolve("../../etc/passwd");
        assert!(result.is_err(), "path traversal with .. must be blocked");
    }

    #[test]
    fn resolve_dot_path_ok() {
        let exec = TauriToolExecutor::new(PathBuf::from("/tmp"));
        let result = exec.resolve(".");
        assert!(result.is_ok());
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
            ToolCall::Think { thought } => {
                ToolResult::ok("think", format!("Reasoning noted ({} chars).", thought.len()))
            }
        }
    }
}
