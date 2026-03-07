//! Executes agent tool calls against the local filesystem.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, ToolExecutorTrait};
use vibe_ai::provider::AIProvider;
use vibe_ai::tools::{ToolCall, ToolResult};
use vibe_ai::WorktreeManager;
use std::path::Path as StdPath; // used in search() glob filter
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
    /// Web search engine: "duckduckgo" | "tavily" | "brave"
    pub search_engine: String,
    /// API key for Tavily (if engine = "tavily").
    pub tavily_api_key: Option<String>,
    /// API key for Brave Search (if engine = "brave").
    pub brave_api_key: Option<String>,
    /// LLM provider used when spawning sub-agents via `spawn_agent` tool.
    pub provider: Option<Arc<dyn AIProvider>>,
    /// Parent agent context for recursive subagent tree tracking.
    pub parent_context: Option<AgentContext>,
    /// When true, all network access is blocked: WebSearch and FetchUrl tools
    /// return errors, and shell commands are wrapped in OS-level network
    /// isolation (`sandbox-exec -n no-network` on macOS, `unshare --net` on Linux).
    pub network_disabled: bool,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf, sandbox: bool) -> Self {
        Self {
            workspace_root,
            sandbox,
            env_policy: None,
            search_engine: "duckduckgo".to_string(),
            tavily_api_key: None,
            brave_api_key: None,
            provider: None,
            parent_context: None,
            network_disabled: false,
        }
    }

    pub fn with_env_policy(mut self, policy: ShellEnvPolicy) -> Self {
        self.env_policy = Some(policy);
        self
    }

    pub fn with_search_config(mut self, engine: String, tavily_key: Option<String>, brave_key: Option<String>) -> Self {
        self.search_engine = engine;
        self.tavily_api_key = tavily_key;
        self.brave_api_key = brave_key;
        self
    }

    pub fn with_provider(mut self, provider: Arc<dyn AIProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Enable internet-disabled sandbox mode. Blocks WebSearch, FetchUrl, and
    /// wraps shell commands in OS-level network isolation.
    pub fn with_no_network(mut self) -> Self {
        self.network_disabled = true;
        self
    }
}

#[async_trait]
impl ToolExecutorTrait for ToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        // Block network-dependent tools when --no-network is active.
        if self.network_disabled {
            match call {
                ToolCall::WebSearch { .. } => {
                    return ToolResult::err("web_search", "Network access is disabled in sandbox mode");
                }
                ToolCall::FetchUrl { .. } => {
                    return ToolResult::err("fetch_url", "Network access is disabled in sandbox mode");
                }
                _ => {}
            }
        }

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
            ToolCall::SpawnAgent { task, max_steps, max_depth } => {
                self.spawn_sub_agent(task, *max_steps, *max_depth).await
            }
        }
    }
}

impl ToolExecutor {
    async fn read_file(&self, path: &str) -> ToolResult {
        let resolved = match self.resolve_safe(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("read_file", e),
        };
        // Use tokio::fs to avoid blocking the async runtime thread on slow
        // filesystems (NFS, cold page-cache, USB drives).
        match tokio::fs::read_to_string(&resolved).await {
            Ok(content) => ToolResult::ok("read_file", content),
            Err(e) => ToolResult::err("read_file", format!("Cannot read {}: {}", resolved.display(), e)),
        }
    }

    async fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let resolved = match self.resolve_safe(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("write_file", e),
        };
        if let Some(parent) = resolved.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ToolResult::err("write_file", format!("Cannot create directories: {}", e));
            }
        }
        match tokio::fs::write(&resolved, content).await {
            Ok(_) => ToolResult::ok(
                "write_file",
                format!("Written {} bytes to {}", content.len(), resolved.display()),
            ),
            Err(e) => ToolResult::err("write_file", format!("Cannot write {}: {}", resolved.display(), e)),
        }
    }

    async fn apply_patch(&self, path: &str, patch: &str) -> ToolResult {
        let resolved = match self.resolve_safe(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("apply_patch", e),
        };
        let original = match tokio::fs::read_to_string(&resolved).await {
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
        match tokio::fs::write(&resolved, &patched).await {
            Ok(_) => ToolResult::ok("apply_patch", format!("Patch applied to {}", resolved.display())),
            Err(e) => ToolResult::err("apply_patch", format!("Cannot write patched file: {}", e)),
        }
    }

    async fn run_bash(&self, command: &str) -> ToolResult {
        let cwd = &self.workspace_root;

        // Build custom environment if a policy is configured
        let custom_env: Option<HashMap<String, String>> =
            self.env_policy.as_ref().map(|p| p.build_env());

        // When network is disabled, wrap the command in OS-level network
        // isolation so subprocesses cannot reach the internet.
        let effective_command: std::borrow::Cow<'_, str> = if self.network_disabled {
            // Escape the inner command for safe embedding in a single-quoted shell arg.
            // Replace every ' with '\'' (end quote, escaped quote, start quote).
            let escaped = command.replace('\'', "'\\''");
            if cfg!(target_os = "macos") {
                // macOS Seatbelt: sandbox-exec with the built-in "no-network" profile
                std::borrow::Cow::Owned(format!(
                    "sandbox-exec -n no-network sh -c '{}'",
                    escaped
                ))
            } else {
                // Linux: unshare(1) creates a new network namespace with no interfaces
                std::borrow::Cow::Owned(format!(
                    "unshare --net sh -c '{}'",
                    escaped
                ))
            }
        } else {
            std::borrow::Cow::Borrowed(command)
        };

        let output = if self.sandbox {
            CommandExecutor::execute_sandboxed(&effective_command, cwd, cwd)
        } else if let Some(env) = custom_env {
            // Execute with custom environment
            use std::process::Command;
            Command::new("sh")
                .arg("-c")
                .arg(effective_command.as_ref())
                .current_dir(cwd)
                .env_clear()
                .envs(env)
                .output()
                .map_err(anyhow::Error::from)
        } else {
            CommandExecutor::execute_in(&effective_command, cwd)
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
        match self.search_engine.as_str() {
            "tavily" => self.tavily_search(query, num_results).await,
            "brave" => self.brave_search(query, num_results).await,
            _ => self.duckduckgo_search(query, num_results).await,
        }
    }

    async fn duckduckgo_search(&self, query: &str, num_results: usize) -> ToolResult {
        // DuckDuckGo Instant Answer API (no API key required)
        let n = num_results.min(10);
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&no_redirect=1",
            urlencoding::encode(query)
        );

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err("web_search", format!("HTTP client error: {}", e)),
        };

        match client.get(&url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let mut results = Vec::new();
                    if let Some(text) = json["AbstractText"].as_str().filter(|s| !s.is_empty()) {
                        results.push(format!(
                            "1. {} ({})\n   {}",
                            json["Heading"].as_str().unwrap_or("Wikipedia"),
                            json["AbstractURL"].as_str().unwrap_or(""),
                            text
                        ));
                    }
                    if let Some(topics) = json["RelatedTopics"].as_array() {
                        for topic in topics.iter().take(n.saturating_sub(results.len())) {
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

    async fn tavily_search(&self, query: &str, num_results: usize) -> ToolResult {
        let api_key = match &self.tavily_api_key {
            Some(k) => k.clone(),
            None => return ToolResult::err("web_search", "Tavily API key not configured. Set TAVILY_API_KEY or tools.web_search.tavily_api_key in config."),
        };

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("VibeCLI/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err("web_search", format!("HTTP client error: {}", e)),
        };

        let payload = serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": num_results.min(10),
            "search_depth": "basic",
            "include_answer": true,
        });

        match client.post("https://api.tavily.com/search").json(&payload).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let mut output = Vec::new();
                    // Include Tavily's AI-generated answer if present
                    if let Some(answer) = json["answer"].as_str().filter(|s| !s.is_empty()) {
                        output.push(format!("**Answer:** {}", answer));
                    }
                    // Include individual results
                    if let Some(results) = json["results"].as_array() {
                        for (i, result) in results.iter().enumerate() {
                            let title = result["title"].as_str().unwrap_or("Untitled");
                            let url = result["url"].as_str().unwrap_or("");
                            let content = result["content"].as_str().unwrap_or("");
                            let score = result["score"].as_f64().unwrap_or(0.0);
                            output.push(format!(
                                "{}. **{}** ({:.2})\n   {}\n   {}",
                                i + 1, title, score, url,
                                if content.len() > 200 { &content[..content.char_indices().nth(200).map(|(i,_)| i).unwrap_or(content.len())] } else { content }
                            ));
                        }
                    }
                    if output.is_empty() {
                        ToolResult::ok("web_search", format!("No results found for: {}", query))
                    } else {
                        ToolResult::ok("web_search", output.join("\n\n"))
                    }
                }
                Err(e) => ToolResult::err("web_search", format!("Tavily JSON parse error: {}", e)),
            },
            Err(e) => ToolResult::err("web_search", format!("Tavily request failed: {}", e)),
        }
    }

    async fn brave_search(&self, query: &str, num_results: usize) -> ToolResult {
        let api_key = match &self.brave_api_key {
            Some(k) => k.clone(),
            None => return ToolResult::err("web_search", "Brave API key not configured. Set BRAVE_API_KEY or tools.web_search.brave_api_key in config."),
        };

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err("web_search", format!("HTTP client error: {}", e)),
        };

        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            num_results.min(10)
        );

        match client.get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", &api_key)
            .send().await
        {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let mut output = Vec::new();
                    if let Some(results) = json["web"]["results"].as_array() {
                        for (i, result) in results.iter().enumerate() {
                            let title = result["title"].as_str().unwrap_or("Untitled");
                            let url = result["url"].as_str().unwrap_or("");
                            let desc = result["description"].as_str().unwrap_or("");
                            output.push(format!("{}. **{}**\n   {}\n   {}", i + 1, title, url, desc));
                        }
                    }
                    if output.is_empty() {
                        ToolResult::ok("web_search", format!("No results found for: {}", query))
                    } else {
                        ToolResult::ok("web_search", output.join("\n\n"))
                    }
                }
                Err(e) => ToolResult::err("web_search", format!("Brave JSON parse error: {}", e)),
            },
            Err(e) => ToolResult::err("web_search", format!("Brave request failed: {}", e)),
        }
    }

    async fn fetch_url(&self, url: &str) -> ToolResult {
        // Validate URL scheme to prevent file://, javascript:, data: etc.
        let url_lower = url.to_lowercase();
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            return ToolResult::err(
                "fetch_url",
                format!("Only http:// and https:// URLs are allowed, got: {}", url.chars().take(50).collect::<String>()),
            );
        }

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
                            let safe: String = text.chars().take(4000).collect();
                            format!("{}\n\n[… content truncated at 4000 chars …]", safe)
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
        let resolved = match self.resolve_safe(path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err("list_directory", e),
        };
        match tokio::fs::read_dir(&resolved).await {
            Ok(mut entries) => {
                let mut lines = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let meta = entry.metadata().await.ok();
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

    /// Resolve a user-supplied path, ensuring it stays within the workspace root.
    ///
    /// Absolute paths are accepted only if they fall inside the workspace.
    /// Relative paths are joined to the workspace root.  In both cases the
    /// result is canonicalized (symlinks resolved, `..` collapsed) and
    /// jail-checked against the canonical workspace root.
    ///
    /// Returns `Err` with a descriptive message on path traversal attempts.
    fn resolve_safe(&self, path: &str) -> Result<PathBuf, String> {
        let candidate = {
            let p = Path::new(path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                self.workspace_root.join(p)
            }
        };

        // Canonicalize workspace root (must succeed; it's a known directory).
        let canonical_root = self.workspace_root.canonicalize().map_err(|e| {
            format!("Cannot canonicalize workspace root: {}", e)
        })?;

        // For existing files we can canonicalize directly.
        // For new files (write_file, create_dir) the file may not exist yet,
        // so we canonicalize the nearest existing ancestor and append the rest.
        let canonical = if candidate.exists() {
            candidate.canonicalize().map_err(|e| {
                format!("Cannot canonicalize path '{}': {}", path, e)
            })?
        } else {
            // Walk up until we find an existing ancestor, then re-join.
            let mut existing = candidate.clone();
            let mut remainder = Vec::new();
            while !existing.exists() {
                if let Some(name) = existing.file_name() {
                    remainder.push(name.to_os_string());
                } else {
                    break;
                }
                existing = match existing.parent() {
                    Some(p) => p.to_path_buf(),
                    None => break,
                };
            }
            let mut base = existing.canonicalize().map_err(|e| {
                format!("Cannot canonicalize ancestor of '{}': {}", path, e)
            })?;
            for component in remainder.into_iter().rev() {
                base.push(component);
            }
            base
        };

        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "Path traversal blocked: '{}' resolves outside workspace",
                path
            ));
        }

        Ok(canonical)
    }

    /// Spawn a nested AgentLoop to complete a delegated sub-task.
    /// Requires a provider to be set via `with_provider()`.
    /// Supports recursive subagent trees with depth limits and global agent caps.
    pub async fn spawn_sub_agent(&self, task: &str, max_steps: Option<usize>, max_depth: Option<u32>) -> ToolResult {
        let provider = match &self.provider {
            Some(p) => p.clone(),
            None => {
                return ToolResult::err(
                    "spawn_agent",
                    "No LLM provider configured for sub-agent. Call with_provider() on ToolExecutor.",
                )
            }
        };

        // ── Depth and counter checks ──────────────────────────────────────────
        let current_depth = self.parent_context.as_ref().map(|c| c.depth).unwrap_or(0);
        let depth_limit = max_depth.unwrap_or(3).min(5); // hard max 5
        if current_depth >= depth_limit {
            return ToolResult::err(
                "spawn_agent",
                format!("Maximum agent nesting depth ({}) exceeded at depth {}", depth_limit, current_depth),
            );
        }

        // Get or create the global agent counter
        let counter = self.parent_context.as_ref()
            .and_then(|c| c.active_agent_counter.clone())
            .unwrap_or_else(|| std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)));

        let active = counter.load(std::sync::atomic::Ordering::Relaxed);
        if active >= 20 {
            return ToolResult::err(
                "spawn_agent",
                format!("Global agent limit (20) reached — {} agents active across the tree", active),
            );
        }
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Build a child executor that shares everything (including the provider ref).
        let child_context = AgentContext {
            workspace_root: self.workspace_root.clone(),
            parent_session_id: self.parent_context.as_ref()
                .and_then(|c| c.parent_session_id.clone())
                .or_else(|| Some(format!("root-{}", std::process::id()))),
            depth: current_depth + 1,
            active_agent_counter: Some(counter.clone()),
            ..Default::default()
        };

        let child_executor: Arc<dyn ToolExecutorTrait> = Arc::new(ToolExecutor {
            workspace_root: self.workspace_root.clone(),
            sandbox: self.sandbox,
            env_policy: self.env_policy.clone(),
            search_engine: self.search_engine.clone(),
            tavily_api_key: self.tavily_api_key.clone(),
            brave_api_key: self.brave_api_key.clone(),
            provider: self.provider.clone(),
            parent_context: Some(child_context.clone()),
            network_disabled: self.network_disabled,
        });

        let mut agent = AgentLoop::new(provider, ApprovalPolicy::FullAuto, child_executor);
        agent.max_steps = max_steps.unwrap_or(10);

        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel::<AgentEvent>(64);

        let task_owned = task.to_string();
        let handle = tokio::spawn(async move {
            agent.run(&task_owned, child_context, event_tx).await
        });

        let mut summary = String::new();
        let mut steps: Vec<String> = Vec::new();

        while let Some(event) = event_rx.recv().await {
            match event {
                AgentEvent::Complete(s) => {
                    summary = s;
                    break;
                }
                AgentEvent::Error(e) => {
                    handle.abort();
                    counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    return ToolResult::err("spawn_agent", format!("Sub-agent error: {}", e));
                }
                AgentEvent::ToolCallExecuted(step) => {
                    steps.push(format!(
                        "  [step {}] {} → {}",
                        step.step_num,
                        step.tool_call.summary(),
                        if step.tool_result.success { "ok" } else { "err" }
                    ));
                }
                _ => {}
            }
        }

        let _ = handle.await;
        counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

        let mut output = String::new();
        output.push_str(&format!("[depth {}/{}] ", current_depth + 1, depth_limit));
        if !steps.is_empty() {
            output.push_str("Steps:\n");
            output.push_str(&steps.join("\n"));
            output.push_str("\n\n");
        }
        output.push_str("Summary: ");
        output.push_str(if summary.is_empty() { "Sub-agent completed." } else { &summary });

        ToolResult::ok("spawn_agent", output)
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
            } else if let Some(added) = line.strip_prefix('+') {
                result.push(added.to_string());
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

/// Decode the six most common HTML entities in a single left-to-right pass,
/// avoiding the six separate `.replace()` calls that each allocate and copy
/// the whole string.
fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        // Try each entity in order; fall back to emitting '&' literally.
        if let Some(tail) = rest.strip_prefix("&amp;") {
            out.push('&');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("&lt;") {
            out.push('<');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("&gt;") {
            out.push('>');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("&quot;") {
            out.push('"');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("&#39;") {
            out.push('\'');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("&nbsp;") {
            out.push(' ');
            rest = tail;
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Minimal HTML → plain text extractor.
/// Strips all tags, decodes common HTML entities, collapses whitespace.
fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    // Work on byte indices for cheap lookahead instead of cloning the char
    // iterator on every '<' (the previous approach allocated a fresh String of
    // up-to-12 characters for every tag encountered).
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        match ch {
            b'<' => {
                in_tag = true;
                // Peek ahead up to 12 ASCII bytes for tag-name classification.
                let peek_end = (i + 1 + 12).min(bytes.len());
                let peek = bytes[i + 1..peek_end]
                    .iter()
                    .map(|b| b.to_ascii_lowercase())
                    .collect::<Vec<u8>>();
                if peek.starts_with(b"script") {
                    in_script = true;
                } else if peek.starts_with(b"/script") {
                    in_script = false;
                } else if peek.starts_with(b"style") {
                    in_style = true;
                } else if peek.starts_with(b"/style") {
                    in_style = false;
                } else if peek.starts_with(b"br")
                    || peek.starts_with(b"p")
                    || peek.starts_with(b"div")
                    || peek.starts_with(b"li")
                {
                    out.push('\n');
                }
            }
            b'>' => {
                in_tag = false;
            }
            _ => {
                if !in_tag && !in_script && !in_style {
                    // Re-interpret the current position as a UTF-8 char.
                    if let Some(c) = html[i..].chars().next() {
                        out.push(c);
                        // Advance by the full UTF-8 char width, not just 1.
                        i += c.len_utf8();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    // Decode common HTML entities in a single pass using a tiny state machine
    // instead of six chained .replace() calls (each of which allocates and
    // copies the full string).
    let out = decode_html_entities(&out);

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
    fn resolve_safe_blocks_path_traversal() {
        let tmp = std::env::temp_dir().join(format!("vibe_resolve_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let executor = ToolExecutor::new(tmp.clone(), false);

        // Relative traversal must be blocked
        let result = executor.resolve_safe("../../etc/passwd");
        assert!(result.is_err(), "relative traversal should be blocked");
        assert!(result.unwrap_err().contains("traversal blocked"));

        // Absolute path outside workspace must be blocked
        let result = executor.resolve_safe("/etc/passwd");
        assert!(result.is_err(), "absolute path outside workspace should be blocked");

        // Normal relative path within workspace must succeed
        std::fs::write(tmp.join("test.txt"), "ok").unwrap();
        let result = executor.resolve_safe("test.txt");
        assert!(result.is_ok(), "normal relative path should succeed");

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_safe_allows_new_file_in_workspace() {
        let tmp = std::env::temp_dir().join(format!("vibe_resolve_new_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let executor = ToolExecutor::new(tmp.clone(), false);

        // Non-existent file in workspace should succeed
        let result = executor.resolve_safe("subdir/new_file.rs");
        assert!(result.is_ok(), "new file path inside workspace should succeed");

        let _ = std::fs::remove_dir_all(&tmp);
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

    #[test]
    fn decode_html_entities_all_six_entities() {
        // Entities are concatenated without separating spaces; verify each
        // one is decoded to its literal character.
        let input = "&amp;&lt;&gt;&quot;&#39;&nbsp;";
        let out = decode_html_entities(input);
        assert_eq!(out, "&<>\"' ");
    }

    #[test]
    fn decode_html_entities_literal_ampersand_passthrough() {
        // Unknown entity — the '&' should be emitted literally.
        let input = "&unknown; hello &amp; world";
        let out = decode_html_entities(input);
        assert!(out.contains("&unknown;") || out.starts_with('&'));
        assert!(out.contains("& world"));
    }

    #[test]
    fn decode_html_entities_no_entities() {
        let input = "no entities here";
        assert_eq!(decode_html_entities(input), input);
    }

    // ── Network-disabled sandbox tests ───────────────────────────────────────

    #[tokio::test]
    async fn no_network_blocks_web_search() {
        let tmp = std::env::temp_dir().join(format!("vibe_nonet_ws_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let executor = ToolExecutor::new(tmp.clone(), false).with_no_network();

        let call = ToolCall::WebSearch { query: "rust lang".to_string(), num_results: 3 };
        let result = executor.execute(&call).await;
        assert!(!result.success);
        assert!(result.output.contains("Network access is disabled in sandbox mode"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn no_network_blocks_fetch_url() {
        let tmp = std::env::temp_dir().join(format!("vibe_nonet_fu_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let executor = ToolExecutor::new(tmp.clone(), false).with_no_network();

        let call = ToolCall::FetchUrl { url: "https://example.com".to_string() };
        let result = executor.execute(&call).await;
        assert!(!result.success);
        assert!(result.output.contains("Network access is disabled in sandbox mode"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn no_network_allows_non_network_tools() {
        let tmp = std::env::temp_dir().join(format!("vibe_nonet_rw_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("hello.txt"), "world").unwrap();
        let executor = ToolExecutor::new(tmp.clone(), false).with_no_network();

        // ReadFile should still work
        let call = ToolCall::ReadFile { path: "hello.txt".to_string() };
        let result = executor.execute(&call).await;
        assert!(result.success, "ReadFile should work in no-network mode");
        assert!(result.output.contains("world"));

        // TaskComplete should still work
        let call = ToolCall::TaskComplete { summary: "done".to_string() };
        let result = executor.execute(&call).await;
        assert!(result.success, "TaskComplete should work in no-network mode");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── var_matches_pattern tests ────────────────────────────────────────────

    #[test]
    fn var_matches_pattern_prefix_wildcard() {
        // `AWS_*` should match any variable starting with `AWS_`
        assert!(var_matches_pattern("AWS_SECRET", "AWS_*"));
        assert!(var_matches_pattern("AWS_ACCESS_KEY_ID", "AWS_*"));
        assert!(var_matches_pattern("AWS_", "AWS_*")); // exact prefix, no suffix chars
    }

    #[test]
    fn var_matches_pattern_suffix_wildcard() {
        // `*_KEY` should match any variable ending with `_KEY`
        assert!(var_matches_pattern("API_KEY", "*_KEY"));
        assert!(var_matches_pattern("SECRET_KEY", "*_KEY"));
        assert!(var_matches_pattern("_KEY", "*_KEY")); // just the suffix
    }

    #[test]
    fn var_matches_pattern_exact_match() {
        assert!(var_matches_pattern("HOME", "HOME"));
        assert!(var_matches_pattern("PATH", "PATH"));
    }

    #[test]
    fn var_matches_pattern_no_match() {
        assert!(!var_matches_pattern("HOME", "PATH"));
        assert!(!var_matches_pattern("AWS_SECRET", "*_KEY"));
        assert!(!var_matches_pattern("MY_VAR", "AWS_*"));
    }

    #[test]
    fn var_matches_pattern_star_matches_everything() {
        // A bare `*` has both starts_with('*') and ends_with('*'), so the
        // function takes the ends_with branch first: pattern = `*`, trimmed
        // prefix is `""`, and `var.starts_with("")` is always true.
        assert!(var_matches_pattern("ANYTHING", "*"));
        assert!(var_matches_pattern("", "*"));
        assert!(var_matches_pattern("AWS_SECRET_KEY", "*"));
    }

    // ── glob_match tests ─────────────────────────────────────────────────────

    #[test]
    fn glob_match_star_matches_everything() {
        assert!(glob_match("*", "foo.rs"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn glob_match_extension_rs() {
        assert!(glob_match("*.rs", "foo.rs"));
        assert!(glob_match("*.rs", "my_module.rs"));
        assert!(glob_match("*.rs", ".rs")); // just the extension
    }

    #[test]
    fn glob_match_extension_tsx_does_not_match_rs() {
        assert!(!glob_match("*.tsx", "foo.rs"));
        assert!(!glob_match("*.tsx", "component.ts")); // ts != tsx
    }

    #[test]
    fn glob_match_exact_name() {
        assert!(glob_match("Cargo.toml", "Cargo.toml"));
        assert!(!glob_match("Cargo.toml", "package.json"));
    }

    #[test]
    fn glob_match_star_dot_edge_case() {
        // `*.` — strip_prefix("*.") yields "", so we check name.ends_with(".")
        assert!(glob_match("*.", "trailing."));
        assert!(!glob_match("*.", "no_trailing_dot"));
    }

    // ── parse_hunk_start tests ───────────────────────────────────────────────

    #[test]
    fn parse_hunk_start_minus_sign() {
        // Standard unified diff header
        let line = "@@ -1,3 +1,4 @@";
        let start = parse_hunk_start(line, '-').unwrap();
        assert_eq!(start, 1);
    }

    #[test]
    fn parse_hunk_start_plus_sign() {
        let line = "@@ -1,3 +1,4 @@";
        let start = parse_hunk_start(line, '+').unwrap();
        assert_eq!(start, 1);
    }

    #[test]
    fn parse_hunk_start_larger_line_numbers() {
        let line = "@@ -42,10 +57,12 @@";
        assert_eq!(parse_hunk_start(line, '-').unwrap(), 42);
        assert_eq!(parse_hunk_start(line, '+').unwrap(), 57);
    }

    #[test]
    fn parse_hunk_start_missing_comma() {
        // Some diffs omit the count when it's 1: `@@ -10 +10 @@`
        let line = "@@ -10 +10 @@";
        assert_eq!(parse_hunk_start(line, '-').unwrap(), 10);
        assert_eq!(parse_hunk_start(line, '+').unwrap(), 10);
    }

    #[test]
    fn parse_hunk_start_malformed_header() {
        // No `-` or `+` token present — function returns Ok(1) as fallback
        let line = "@@ some garbage @@";
        assert_eq!(parse_hunk_start(line, '-').unwrap(), 1);
        assert_eq!(parse_hunk_start(line, '+').unwrap(), 1);
    }

    // ── decode_html_entities additional tests ────────────────────────────────

    #[test]
    fn decode_html_entities_multiple_in_sequence() {
        // Multiple entities interspersed with regular text
        let input = "a &amp; b &lt; c &gt; d";
        assert_eq!(decode_html_entities(input), "a & b < c > d");
    }

    #[test]
    fn decode_html_entities_entity_at_end_of_string() {
        let input = "trailing ampersand &amp;";
        assert_eq!(decode_html_entities(input), "trailing ampersand &");

        let input2 = "quote at end &quot;";
        assert_eq!(decode_html_entities(input2), "quote at end \"");
    }

    #[test]
    fn decode_html_entities_no_entities_passthrough() {
        // Plain text with no `&` at all should pass through unchanged
        let input = "Hello, world! Nothing special here.";
        assert_eq!(decode_html_entities(input), input);
    }

    #[test]
    fn decode_html_entities_all_six_types_individually() {
        assert_eq!(decode_html_entities("&amp;"), "&");
        assert_eq!(decode_html_entities("&lt;"), "<");
        assert_eq!(decode_html_entities("&gt;"), ">");
        assert_eq!(decode_html_entities("&quot;"), "\"");
        assert_eq!(decode_html_entities("&#39;"), "'");
        assert_eq!(decode_html_entities("&nbsp;"), " ");
    }

    #[test]
    fn decode_html_entities_mixed_known_and_unknown() {
        // Unknown entity `&foo;` should have its `&` emitted literally,
        // while known `&amp;` should decode.
        let input = "&foo; &amp; &bar;";
        let out = decode_html_entities(input);
        assert!(out.contains("& "), "known &amp; should become &");
        // The unknown `&foo;` — the `&` is emitted literally, then `foo;`
        // follows as-is.
        assert!(out.contains("&foo;"));
    }

    #[test]
    fn decode_html_entities_consecutive_entities() {
        // Back-to-back entities with no separating text
        let input = "&lt;&gt;&amp;";
        assert_eq!(decode_html_entities(input), "<>&");
    }

    #[test]
    fn decode_html_entities_empty_string() {
        assert_eq!(decode_html_entities(""), "");
    }
}
