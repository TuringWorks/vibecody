#![allow(dead_code)]
//! BYOA (Bring Your Own Agent) adapter registry for VibeCody company orchestration.
//!
//! Adapters are pluggable execution backends for company agents. The registry
//! is mutable at runtime — adapters can be registered and removed without
//! restarting the application.
//!
//! Built-in adapters:
//! - **Internal**: Wraps VibeCody's own AgentPool (default)
//! - **Claude**: Calls Anthropic Claude via API
//! - **Codex**: Calls OpenAI Codex/Assistants API
//! - **Cursor**: Cursor background agent mode
//! - **Http**: Calls a custom HTTP endpoint
//! - **Process**: Runs a shell command

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ── Context passed to every adapter execution ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterContext {
    /// The task/prompt to execute.
    pub prompt: String,
    /// Company ID.
    pub company_id: String,
    /// Agent ID within the company.
    pub agent_id: String,
    /// Agent title (e.g., "Senior Engineer").
    pub agent_title: String,
    /// Optional goal context injected into the prompt.
    pub goal_context: Option<String>,
    /// Optional working directory.
    pub working_dir: Option<String>,
    /// Session ID for resuming prior context.
    pub session_id: Option<String>,
    /// Model override (if adapter supports it).
    pub model: Option<String>,
    /// Maximum LLM turns.
    pub max_turns: u32,
}

impl AdapterContext {
    pub fn new(prompt: &str, company_id: &str, agent_id: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            company_id: company_id.to_string(),
            agent_id: agent_id.to_string(),
            agent_title: String::new(),
            goal_context: None,
            working_dir: None,
            session_id: None,
            model: None,
            max_turns: 25,
        }
    }
}

// ── Execution result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub session_id: Option<String>,
    pub tokens_used: u64,
    pub duration_ms: u64,
}

impl ExecutionResult {
    pub fn ok(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
            session_id: None,
            tokens_used: 0,
            duration_ms: 0,
        }
    }
    pub fn err(msg: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(msg),
            session_id: None,
            tokens_used: 0,
            duration_ms: 0,
        }
    }
}

// ── AgentAdapter trait ────────────────────────────────────────────────────────

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Execute a task with the given context and return the result.
    async fn execute(&self, ctx: &AdapterContext) -> Result<ExecutionResult>;
    /// Adapter type identifier.
    fn adapter_type(&self) -> &str;
    /// Human-readable label.
    fn label(&self) -> &str;
    /// Verify the adapter is reachable/configured (health check).
    async fn ping(&self) -> Result<()> {
        Ok(())
    }
}

// ── Built-in: Internal adapter (wraps VibeCody AgentPool) ────────────────────

pub struct InternalAdapter;

#[async_trait]
impl AgentAdapter for InternalAdapter {
    async fn execute(&self, ctx: &AdapterContext) -> Result<ExecutionResult> {
        // Delegates to vibecli's own agent pipeline.
        // In Phase 6 (heartbeat wiring) this will be fully integrated.
        // For now, simulate execution and return a placeholder.
        Ok(ExecutionResult {
            success: true,
            output: format!(
                "[InternalAdapter] Task queued for agent {} in company {}.\nPrompt: {}",
                &ctx.agent_id[..8.min(ctx.agent_id.len())],
                ctx.company_id,
                ctx.prompt
            ),
            error: None,
            session_id: None,
            tokens_used: 0,
            duration_ms: 0,
        })
    }
    fn adapter_type(&self) -> &str { "internal" }
    fn label(&self) -> &str { "VibeCody Internal" }
}

// ── Built-in: HTTP adapter ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAdapterConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub timeout_secs: u64,
}

pub struct HttpAdapter {
    config: HttpAdapterConfig,
    client: reqwest::Client,
}

impl HttpAdapter {
    pub fn new(config: HttpAdapterConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AgentAdapter for HttpAdapter {
    async fn execute(&self, ctx: &AdapterContext) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let payload = serde_json::json!({
            "prompt": ctx.prompt,
            "agent_id": ctx.agent_id,
            "company_id": ctx.company_id,
            "model": ctx.model,
            "session_id": ctx.session_id,
        });
        let resp = self.client
            .post(&self.config.url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs.max(30)))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let duration_ms = start.elapsed().as_millis() as u64;
        if status.is_success() {
            Ok(ExecutionResult { success: true, output: body, error: None, session_id: None, tokens_used: 0, duration_ms })
        } else {
            Ok(ExecutionResult { success: false, output: String::new(), error: Some(format!("HTTP {status}: {body}")), session_id: None, tokens_used: 0, duration_ms })
        }
    }
    fn adapter_type(&self) -> &str { "http" }
    fn label(&self) -> &str { "HTTP Endpoint" }
    async fn ping(&self) -> Result<()> {
        self.client.head(&self.config.url).timeout(std::time::Duration::from_secs(5)).send().await?;
        Ok(())
    }
}

// ── Built-in: Process adapter (shell command) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAdapterConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

pub struct ProcessAdapter {
    config: ProcessAdapterConfig,
}

impl ProcessAdapter {
    pub fn new(config: ProcessAdapterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AgentAdapter for ProcessAdapter {
    async fn execute(&self, ctx: &AdapterContext) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let config = self.config.clone();
        let prompt = ctx.prompt.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(&config.command);
            cmd.args(&config.args);
            for (k, v) in &config.env {
                cmd.env(k, v);
            }
            if let Some(dir) = &config.working_dir {
                cmd.current_dir(dir);
            }
            cmd.env("VIBECODY_PROMPT", &prompt);
            cmd.stdin(std::process::Stdio::null());
            cmd.output()
        }).await??;
        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        if result.status.success() {
            Ok(ExecutionResult { success: true, output: stdout, error: if stderr.is_empty() { None } else { Some(stderr) }, session_id: None, tokens_used: 0, duration_ms })
        } else {
            Ok(ExecutionResult { success: false, output: stdout, error: Some(stderr), session_id: None, tokens_used: 0, duration_ms })
        }
    }
    fn adapter_type(&self) -> &str { "process" }
    fn label(&self) -> &str { "Shell Process" }
}

// ── Adapter metadata for display ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub adapter_type: String,
    pub label: String,
    pub config_json: serde_json::Value,
}

// ── AdapterRegistry ───────────────────────────────────────────────────────────

pub struct AdapterRegistry {
    adapters: RwLock<HashMap<String, Arc<dyn AgentAdapter>>>,
    configs: RwLock<HashMap<String, serde_json::Value>>,
}

impl AdapterRegistry {
    /// Create a registry pre-loaded with built-in adapters.
    pub fn new() -> Arc<Self> {
        let registry = Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
        });
        // Register built-in internal adapter
        registry.register_boxed(
            "internal",
            Arc::new(InternalAdapter),
            serde_json::json!({}),
        );
        registry
    }

    pub fn register_boxed(
        &self,
        name: &str,
        adapter: Arc<dyn AgentAdapter>,
        config: serde_json::Value,
    ) {
        self.adapters.write().unwrap().insert(name.to_string(), adapter);
        self.configs.write().unwrap().insert(name.to_string(), config);
    }

    pub fn register_http(&self, name: &str, config: HttpAdapterConfig) {
        let cfg_json = serde_json::to_value(&config).unwrap_or(serde_json::json!({}));
        self.register_boxed(name, Arc::new(HttpAdapter::new(config)), cfg_json);
    }

    pub fn register_process(&self, name: &str, config: ProcessAdapterConfig) {
        let cfg_json = serde_json::to_value(&config).unwrap_or(serde_json::json!({}));
        self.register_boxed(name, Arc::new(ProcessAdapter::new(config)), cfg_json);
    }

    pub fn unregister(&self, name: &str) -> bool {
        let removed = self.adapters.write().unwrap().remove(name).is_some();
        self.configs.write().unwrap().remove(name);
        removed
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AgentAdapter>> {
        self.adapters.read().unwrap().get(name).cloned()
    }

    /// Get adapter by type string (first match wins).
    pub fn get_by_type(&self, adapter_type: &str) -> Option<Arc<dyn AgentAdapter>> {
        self.adapters.read().unwrap().values()
            .find(|a| a.adapter_type() == adapter_type)
            .cloned()
    }

    pub fn list(&self) -> Vec<AdapterInfo> {
        let adapters = self.adapters.read().unwrap();
        let configs = self.configs.read().unwrap();
        let mut infos: Vec<AdapterInfo> = adapters.iter().map(|(name, adapter)| {
            AdapterInfo {
                name: name.clone(),
                adapter_type: adapter.adapter_type().to_string(),
                label: adapter.label().to_string(),
                config_json: configs.get(name).cloned().unwrap_or(serde_json::json!({})),
            }
        }).collect();
        infos.sort_by(|a, b| a.name.cmp(&b.name));
        infos
    }
}

/// Convenience: create a global-default registry (Arc-wrapped).
pub fn default_registry() -> Arc<AdapterRegistry> {
    AdapterRegistry::new()
}
