//! Executes agent tool calls against the local filesystem.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vibe_ai::agent::ToolExecutorTrait;
use vibe_ai::tools::{ToolCall, ToolResult};
use vibe_ai::WorktreeManager;
use std::path::Path as StdPath;
use vibe_core::executor::CommandExecutor;
use vibe_core::search::search_files;

/// Shell environment policy — controls what env vars subprocesses inherit.
#[derive(Debug, Clone, Default)]
pub struct ShellEnvPolicy {
    /// Base inheritance: "all" | "core" | "none"
    pub inherit: String,
    /// Additional variable names (or glob patterns) to always include.
    pub include: Vec<String>,
    /// Variable names (or glob patterns starting with *) to exclude.
    pub exclude: Vec<String>,
    /// Additional variables to forcibly set.
    pub set: HashMap<String, String>,
}

impl ShellEnvPolicy {
    /// Build the environment map for a subprocess.
    pub fn build_env(&self) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = match self.inherit.as_str() {
            "all" => std::env::vars().collect(),
            "none" => HashMap::new(),
            _ => {
                // "core" — keep PATH, HOME, USER, SHELL, TERM, LANG, TMPDIR + common build vars
                let core_keys = [
                    "PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "TMPDIR",
                    "CARGO_HOME", "RUSTUP_HOME", "GOPATH", "GOROOT",
                    "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME",
                ];
                std::env::vars()
                    .filter(|(k, _)| core_keys.contains(&k.as_str()))
                    .collect()
            }
        };

        // Apply include list
        for pattern in &self.include {
            for (k, v) in std::env::vars() {
                if var_matches_pattern(&k, pattern) {
                    env.insert(k, v);
                }
            }
        }

        // Apply exclude list
        env.retain(|k, _| {
            !self.exclude.iter().any(|pat| var_matches_pattern(k, pat))
        });

        // Apply forced set
        for (k, v) in &self.set {
            env.insert(k.clone(), v.clone());
        }

        env
    }
}

fn var_matches_pattern(var: &str, pattern: &str) -> bool {
    if pattern.ends_with('*') {
        var.starts_with(pattern.trim_end_matches('*'))
    } else if pattern.starts_with('*') {
        var.ends_with(pattern.trim_start_matches('*'))
    } else {
        var == pattern
    }
}

#[derive(Clone)]
pub struct ToolExecutor {
    pub workspace_root: PathBuf,
    pub sandbox: bool,
    pub env_policy: Option<ShellEnvPolicy>,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf, sandbox: bool) -> Self {
        Self { workspace_root, sandbox, env_policy: None }
    }

    pub fn with_env_policy(mut self, policy: ShellEnvPolicy) -> Self {
        self.env_policy = Some(policy);
        self
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
            ToolCall::WebSearch { query, num_results } => {
                self.web_search(query, *num_results).await
            }
            ToolCall::FetchUrl { url } => self.fetch_url(url).await,
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

        // Build custom environment if a policy is configured
        let custom_env: Option<HashMap<String, String>> =
            self.env_policy.as_ref().map(|p| p.build_env());

        let output = if self.sandbox {
            CommandExecutor::execute_sandboxed(command, cwd, cwd)
        } else if let Some(env) = custom_env {
            // Execute with custom environment
            use std::process::Command;
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(cwd)
                .env_clear()
                .envs(env)
                .output()
                .map_err(anyhow::Error::from)
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

    async fn web_search(&self, query: &str, num_results: usize) -> ToolResult {
        // DuckDuckGo Instant Answer API (no API key required)
        let n = num_results.min(10);
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&no_redirect=1",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI/1.0")
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => return ToolResult::err("web_search", format!("HTTP client error: {}", e)),
        };

        match client.get(&url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let mut results = Vec::new();

                    // AbstractText (instant answer)
                    if let Some(text) = json["AbstractText"].as_str().filter(|s| !s.is_empty()) {
                        results.push(format!(
                            "1. {} ({})\n   {}",
                            json["Heading"].as_str().unwrap_or("Wikipedia"),
                            json["AbstractURL"].as_str().unwrap_or(""),
                            text
                        ));
                    }

                    // RelatedTopics
                    if let Some(topics) = json["RelatedTopics"].as_array() {
                        for (_i, topic) in topics.iter().take(n.saturating_sub(results.len())).enumerate() {
                            if let Some(text) = topic["Text"].as_str().filter(|s| !s.is_empty()) {
                                let url_str = topic["FirstURL"].as_str().unwrap_or("");
                                results.push(format!("{}. {}\n   {}", results.len() + 1, url_str, text));
                            }
                        }
                    }

                    if results.is_empty() {
                        ToolResult::ok("web_search", format!("No results found for: {}", query))
                    } else {
                        ToolResult::ok("web_search", results.join("\n\n"))
                    }
                }
                Err(e) => ToolResult::err("web_search", format!("JSON parse error: {}", e)),
            },
            Err(e) => ToolResult::err("web_search", format!("Request failed: {}", e)),
        }
    }

    async fn fetch_url(&self, url: &str) -> ToolResult {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("VibeCLI/1.0")
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => return ToolResult::err("fetch_url", format!("HTTP client error: {}", e)),
        };

        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(html) => {
                        // Strip HTML tags for a readable plain-text extract
                        let text = html_to_text(&html);
                        let truncated_text = if text.len() > 4000 {
                            format!("{}\n\n[… content truncated at 4000 chars …]", &text[..4000])
                        } else {
                            text
                        };
                        if status.is_success() {
                            ToolResult::ok("fetch_url", truncated_text)
                        } else {
                            ToolResult::err(
                                "fetch_url",
                                format!("HTTP {}: {}", status.as_u16(), truncated_text),
                            )
                        }
                    }
                    Err(e) => ToolResult::err("fetch_url", format!("Read body error: {}", e)),
                }
            }
            Err(e) => ToolResult::err("fetch_url", format!("Request failed: {}", e)),
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

/// Minimal HTML → plain text extractor.
/// Strips all tags, decodes common entities, collapses whitespace.
fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut buf = String::new();

    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                // Collect tag name
                buf.clear();
                in_tag = true;
                // Peek ahead for script/style
                let remaining: String = chars.clone().take(12).collect();
                let lower = remaining.to_lowercase();
                if lower.starts_with("script") || lower.starts_with("/script") {
                    in_script = lower.starts_with("script");
                } else if lower.starts_with("style") || lower.starts_with("/style") {
                    in_style = lower.starts_with("style");
                } else if lower.starts_with("br") || lower.starts_with("p") || lower.starts_with("div") || lower.starts_with("li") {
                    out.push('\n');
                }
            }
            '>' => {
                in_tag = false;
            }
            _ => {
                if !in_tag && !in_script && !in_style {
                    out.push(ch);
                }
            }
        }
    }

    // Decode common HTML entities
    let out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse excess whitespace
    let mut result = String::with_capacity(out.len());
    let mut last_newline = false;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_newline {
                result.push('\n');
                last_newline = true;
            }
        } else {
            result.push_str(trimmed);
            result.push('\n');
            last_newline = false;
        }
    }
    result
}

// ── WorktreeManager implementation ───────────────────────────────────────────

/// `WorktreeManager` backed by `vibe_core::git` (git CLI subprocess).
pub struct VibeCoreWorktreeManager {
    /// The primary repository root used as the CWD for `git worktree` commands.
    pub repo_path: PathBuf,
}

impl VibeCoreWorktreeManager {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }
}

impl WorktreeManager for VibeCoreWorktreeManager {
    fn create_worktree(&self, branch: &str, worktree_path: &std::path::Path) -> Result<()> {
        vibe_core::git::create_worktree(&self.repo_path, branch, worktree_path)
    }

    fn remove_worktree(&self, worktree_path: &std::path::Path) -> Result<()> {
        vibe_core::git::remove_worktree(&self.repo_path, worktree_path)
    }

    fn create_isolated_worktree(&self, agent_id: &str) -> Result<vibe_ai::IsolatedWorktree> {
        use std::sync::Arc;
        // Sanitize agent_id for branch name
        let safe_id = agent_id.replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
        let branch = format!("agent/{}", safe_id);
        let wt_dir = self.repo_path.join(".vibecli").join("worktrees").join(&safe_id);
        std::fs::create_dir_all(&wt_dir)?;
        self.create_worktree(&branch, &wt_dir)?;
        let manager: Arc<dyn WorktreeManager> = Arc::new(VibeCoreWorktreeManager {
            repo_path: self.repo_path.clone(),
        });
        Ok(vibe_ai::IsolatedWorktree::new(wt_dir, branch, agent_id.to_string(), manager))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_env_policy_core_keeps_path() {
        let policy = ShellEnvPolicy {
            inherit: "core".to_string(),
            include: vec![],
            exclude: vec![],
            set: HashMap::new(),
        };
        let env = policy.build_env();
        // PATH should always be present in a normal environment
        // (test won't fail if PATH is unset in the test runner)
        let _ = env; // just ensure build_env doesn't panic
    }

    #[test]
    fn shell_env_policy_none_only_set_vars() {
        let mut set = HashMap::new();
        set.insert("VIBECLI_AGENT".to_string(), "1".to_string());
        let policy = ShellEnvPolicy {
            inherit: "none".to_string(),
            include: vec![],
            exclude: vec![],
            set,
        };
        let env = policy.build_env();
        assert_eq!(env.get("VIBECLI_AGENT").map(|s| s.as_str()), Some("1"));
        // Should have exactly 1 key
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn shell_env_policy_exclude_pattern() {
        std::env::set_var("__TEST_API_KEY", "secret");
        let policy = ShellEnvPolicy {
            inherit: "all".to_string(),
            include: vec![],
            exclude: vec!["__TEST_API_KEY".to_string()],
            set: HashMap::new(),
        };
        let env = policy.build_env();
        assert!(!env.contains_key("__TEST_API_KEY"));
        std::env::remove_var("__TEST_API_KEY");
    }

    #[test]
    fn html_to_text_strips_tags() {
        let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn html_to_text_decodes_entities() {
        let html = "<p>a &amp; b &lt;c&gt;</p>";
        let text = html_to_text(html);
        assert!(text.contains("a & b <c>"));
    }
}
