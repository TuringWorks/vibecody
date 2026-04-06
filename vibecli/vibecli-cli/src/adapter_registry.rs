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

#[cfg(test)]
mod tests {
    use super::*;

    // ── new / default ────────────────────────────────────────────────────────

    #[test]
    fn given_new_registry_when_created_then_internal_adapter_pre_registered() {
        let reg = AdapterRegistry::new();
        let adapter = reg.get("internal");
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().adapter_type(), "internal");
    }

    #[test]
    fn given_new_registry_when_list_called_then_contains_internal() {
        let reg = AdapterRegistry::new();
        let list = reg.list();
        assert!(!list.is_empty());
        let names: Vec<&str> = list.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"internal"));
    }

    // ── register_boxed ───────────────────────────────────────────────────────

    #[test]
    fn given_adapter_registered_when_get_then_returned() {
        let reg = AdapterRegistry::new();
        reg.register_boxed("my-internal", Arc::new(InternalAdapter), serde_json::json!({}));
        let found = reg.get("my-internal");
        assert!(found.is_some());
        assert_eq!(found.unwrap().adapter_type(), "internal");
    }

    #[test]
    fn given_adapter_registered_when_list_then_appears_in_list() {
        let reg = AdapterRegistry::new();
        reg.register_boxed("custom-1", Arc::new(InternalAdapter), serde_json::json!({"x": 1}));
        let list = reg.list();
        let found = list.iter().find(|i| i.name == "custom-1");
        assert!(found.is_some());
        let info = found.unwrap();
        assert_eq!(info.config_json["x"], 1);
    }

    // ── unregister ───────────────────────────────────────────────────────────

    #[test]
    fn given_registered_adapter_when_unregistered_then_returns_true_and_not_found() {
        let reg = AdapterRegistry::new();
        reg.register_boxed("to-remove", Arc::new(InternalAdapter), serde_json::json!({}));
        let removed = reg.unregister("to-remove");
        assert!(removed);
        assert!(reg.get("to-remove").is_none());
    }

    #[test]
    fn given_nonexistent_name_when_unregistered_then_returns_false() {
        let reg = AdapterRegistry::new();
        let removed = reg.unregister("does-not-exist");
        assert!(!removed);
    }

    #[test]
    fn given_unregistered_adapter_when_list_called_then_not_in_list() {
        let reg = AdapterRegistry::new();
        reg.register_boxed("temp", Arc::new(InternalAdapter), serde_json::json!({}));
        reg.unregister("temp");
        let list = reg.list();
        assert!(list.iter().all(|i| i.name != "temp"));
    }

    // ── get ──────────────────────────────────────────────────────────────────

    #[test]
    fn given_nonexistent_name_when_get_then_returns_none() {
        let reg = AdapterRegistry::new();
        let result = reg.get("phantom-adapter");
        assert!(result.is_none());
    }

    // ── get_by_type ──────────────────────────────────────────────────────────

    #[test]
    fn given_internal_adapter_when_get_by_type_internal_then_returned() {
        let reg = AdapterRegistry::new();
        let found = reg.get_by_type("internal");
        assert!(found.is_some());
        assert_eq!(found.unwrap().adapter_type(), "internal");
    }

    #[test]
    fn given_nonexistent_type_when_get_by_type_then_returns_none() {
        let reg = AdapterRegistry::new();
        let result = reg.get_by_type("nonexistent-type");
        assert!(result.is_none());
    }

    // ── register_process ────────────────────────────────────────────────────

    #[test]
    fn given_process_adapter_registered_when_get_then_adapter_type_is_process() {
        let reg = AdapterRegistry::new();
        reg.register_process("my-proc", ProcessAdapterConfig {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: Default::default(),
            working_dir: None,
        });
        let found = reg.get("my-proc");
        assert!(found.is_some());
        assert_eq!(found.unwrap().adapter_type(), "process");
    }

    // ── list ordering ────────────────────────────────────────────────────────

    #[test]
    fn given_multiple_adapters_when_list_then_sorted_by_name() {
        let reg = AdapterRegistry::new();
        reg.register_boxed("zebra", Arc::new(InternalAdapter), serde_json::json!({}));
        reg.register_boxed("alpha", Arc::new(InternalAdapter), serde_json::json!({}));
        let list = reg.list();
        // Verify list is sorted alphabetically
        let names: Vec<&str> = list.iter().map(|i| i.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    // ── label / adapter_type in AdapterInfo ──────────────────────────────────

    #[test]
    fn given_internal_adapter_when_info_fetched_then_label_and_type_correct() {
        let reg = AdapterRegistry::new();
        let list = reg.list();
        let internal_info = list.iter().find(|i| i.name == "internal").unwrap();
        assert_eq!(internal_info.adapter_type, "internal");
        assert_eq!(internal_info.label, "VibeCody Internal");
    }

    // ── AdapterContext helpers ───────────────────────────────────────────────

    #[test]
    fn given_adapter_context_new_when_constructed_then_defaults_are_sensible() {
        let ctx = AdapterContext::new("Do the thing", "co1", "ag1");
        assert_eq!(ctx.prompt, "Do the thing");
        assert_eq!(ctx.company_id, "co1");
        assert_eq!(ctx.agent_id, "ag1");
        assert_eq!(ctx.max_turns, 25);
        assert!(ctx.model.is_none());
        assert!(ctx.session_id.is_none());
    }

    // ── ExecutionResult helpers ──────────────────────────────────────────────

    #[test]
    fn given_execution_result_ok_when_constructed_then_success_true_and_error_none() {
        let r = ExecutionResult::ok("done".to_string());
        assert!(r.success);
        assert_eq!(r.output, "done");
        assert!(r.error.is_none());
    }

    #[test]
    fn given_execution_result_err_when_constructed_then_success_false_and_error_set() {
        let r = ExecutionResult::err("oops".to_string());
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("oops"));
        assert!(r.output.is_empty());
    }
}
