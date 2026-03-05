#![allow(dead_code)]
//! OpenSandbox Client SDK
//!
//! Client for the OpenSandbox managed sandbox infrastructure.
//! Implements both the Lifecycle API and the Execd (execution) API,
//! plus a [`ContainerRuntime`] adapter for unified access.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::container_runtime::*;

// ── Lifecycle API Types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateSandboxRequest {
    pub image: Option<String>,
    #[serde(rename = "keepAliveTimeSeconds")]
    pub keep_alive_time_seconds: Option<u64>,
    pub env: Option<HashMap<String, String>>,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct SandboxResponse {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "execdUrl", default)]
    pub execd_url: Option<String>,
    #[serde(rename = "accessToken", default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SandboxListResponse {
    pub sandboxes: Vec<SandboxResponse>,
}

// ── Execd API Types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RunCommandRequest {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ExecdEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub id: Option<String>,
}

// ── Lifecycle Client ────────────────────────────────────────────────────────

/// Client for the OpenSandbox lifecycle API (create/list/delete sandboxes).
pub struct OpenSandboxClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl OpenSandboxClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        }
    }

    fn auth_header(&self) -> Option<(String, String)> {
        self.api_key
            .as_ref()
            .map(|k| ("OPEN-SANDBOX-API-KEY".to_string(), k.clone()))
    }

    /// Check if the OpenSandbox server is reachable.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/v1/sandboxes", self.base_url);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        req.send()
            .await
            .map(|r| r.status().is_success() || r.status().as_u16() == 401)
            .unwrap_or(false)
    }

    /// Create a new sandbox.
    pub async fn create_sandbox(
        &self,
        request: &CreateSandboxRequest,
    ) -> anyhow::Result<SandboxResponse> {
        let url = format!("{}/v1/sandboxes", self.base_url);
        let mut req = self.client.post(&url).json(request);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("create_sandbox failed ({status}): {body}");
        }
        Ok(resp.json().await?)
    }

    /// List all sandboxes, optionally filtered by state.
    pub async fn list_sandboxes(
        &self,
        state: Option<&str>,
    ) -> anyhow::Result<Vec<SandboxResponse>> {
        let mut url = format!("{}/v1/sandboxes", self.base_url);
        if let Some(s) = state {
            url.push_str(&format!("?state={s}"));
        }
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("list_sandboxes failed ({status}): {body}");
        }
        let list: SandboxListResponse = resp.json().await?;
        Ok(list.sandboxes)
    }

    /// Get sandbox details.
    pub async fn get_sandbox(&self, id: &str) -> anyhow::Result<SandboxResponse> {
        let url = format!("{}/v1/sandboxes/{id}", self.base_url);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("get_sandbox failed ({status}): {body}");
        }
        Ok(resp.json().await?)
    }

    /// Delete (stop + remove) a sandbox.
    pub async fn delete_sandbox(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("{}/v1/sandboxes/{id}", self.base_url);
        let mut req = self.client.delete(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("delete_sandbox failed ({status}): {body}");
        }
        Ok(())
    }

    /// Pause a sandbox.
    pub async fn pause_sandbox(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("{}/v1/sandboxes/{id}/pause", self.base_url);
        let mut req = self.client.post(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("pause_sandbox failed ({status}): {body}");
        }
        Ok(())
    }

    /// Resume a paused sandbox.
    pub async fn resume_sandbox(&self, id: &str) -> anyhow::Result<()> {
        let url = format!("{}/v1/sandboxes/{id}/resume", self.base_url);
        let mut req = self.client.post(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("resume_sandbox failed ({status}): {body}");
        }
        Ok(())
    }
}

// ── Execd Client ────────────────────────────────────────────────────────────

/// Client for the OpenSandbox execd API (run commands, file ops inside a sandbox).
pub struct ExecdClient {
    base_url: String,
    access_token: Option<String>,
    client: Client,
}

impl ExecdClient {
    pub fn new(execd_url: String, access_token: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            base_url: execd_url.trim_end_matches('/').to_string(),
            access_token,
            client,
        }
    }

    fn auth_header(&self) -> Option<(String, String)> {
        self.access_token
            .as_ref()
            .map(|t| ("X-EXECD-ACCESS-TOKEN".to_string(), t.clone()))
    }

    /// Run a command and collect all output via SSE.
    pub async fn run_command(
        &self,
        request: &RunCommandRequest,
    ) -> anyhow::Result<ExecResult> {
        let url = format!("{}/command", self.base_url);
        let mut req = self.client.post(&url).json(request);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("run_command failed ({status}): {body}");
        }

        let body = resp.text().await?;
        let (stdout, stderr, exit_code) = parse_sse_events(&body);

        Ok(ExecResult {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Run a command with streaming output via SSE.
    pub async fn run_command_stream(
        &self,
        request: &RunCommandRequest,
        tx: mpsc::Sender<ExecStreamEvent>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/command", self.base_url);
        let mut req = self.client.post(&url).json(request);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("run_command_stream failed ({status}): {body}");
        }

        let body = resp.text().await?;
        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(event) = serde_json::from_str::<ExecdEvent>(data) {
                    match event.event_type.as_str() {
                        "stdout" => {
                            if let Some(text) = event.text {
                                let _ = tx.send(ExecStreamEvent::Stdout(text)).await;
                            }
                        }
                        "stderr" => {
                            if let Some(text) = event.text {
                                let _ = tx.send(ExecStreamEvent::Stderr(text)).await;
                            }
                        }
                        "result" | "execution_complete" => {
                            if let Some(code) = event.exit_code {
                                let _ = tx.send(ExecStreamEvent::ExitCode(code)).await;
                            }
                        }
                        "error" => {
                            if let Some(text) = event.text {
                                let _ = tx.send(ExecStreamEvent::Error(text)).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Read a file from the sandbox.
    pub async fn download_file(&self, path: &str) -> anyhow::Result<String> {
        let url = format!("{}/file?path={}", self.base_url, urlencoding_encode(path));
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("download_file failed ({status}): {body}");
        }
        Ok(resp.text().await?)
    }

    /// Write content to a file in the sandbox.
    pub async fn upload_file(&self, path: &str, content: &str) -> anyhow::Result<()> {
        let url = format!("{}/file", self.base_url);
        let mut req = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "path": path,
                "content": content,
            }));
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("upload_file failed ({status}): {body}");
        }
        Ok(())
    }

    /// List directory contents.
    pub async fn list_dir(&self, path: &str) -> anyhow::Result<Vec<String>> {
        let url = format!(
            "{}/file/list?path={}",
            self.base_url,
            urlencoding_encode(path)
        );
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("list_dir failed ({status}): {body}");
        }
        let files: Vec<String> = resp.json().await.unwrap_or_default();
        Ok(files)
    }

    /// Get sandbox metrics.
    pub async fn get_metrics(&self) -> anyhow::Result<ContainerMetrics> {
        let url = format!("{}/metrics", self.base_url);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("get_metrics failed");
        }
        let v: serde_json::Value = resp.json().await?;
        Ok(ContainerMetrics {
            cpu_usage_percent: v["cpu_percent"].as_f64().unwrap_or(0.0),
            memory_used_bytes: v["memory_used"].as_u64().unwrap_or(0),
            memory_limit_bytes: v["memory_limit"].as_u64().unwrap_or(0),
            pids: v["pids"].as_u64().unwrap_or(0) as u32,
        })
    }
}

// ── SSE Parser ──────────────────────────────────────────────────────────────

/// Parse SSE event stream text into stdout, stderr, and exit code.
fn parse_sse_events(body: &str) -> (String, String, i32) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_code = 0i32;

    for line in body.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(event) = serde_json::from_str::<ExecdEvent>(data) {
                match event.event_type.as_str() {
                    "stdout" => {
                        if let Some(text) = event.text {
                            stdout.push_str(&text);
                        }
                    }
                    "stderr" => {
                        if let Some(text) = event.text {
                            stderr.push_str(&text);
                        }
                    }
                    "result" | "execution_complete" => {
                        if let Some(code) = event.exit_code {
                            exit_code = code;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (stdout, stderr, exit_code)
}

/// Simple URL encoding for path parameters.
fn urlencoding_encode(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F")
        .replace('&', "%26")
        .replace('+', "%2B")
}

// ── ContainerRuntime Adapter ────────────────────────────────────────────────

/// OpenSandbox adapter implementing [`ContainerRuntime`].
pub struct OpenSandboxRuntime {
    lifecycle: OpenSandboxClient,
    /// Cached execd clients keyed by sandbox ID.
    execd_clients: Arc<Mutex<HashMap<String, ExecdClient>>>,
}

impl OpenSandboxRuntime {
    pub fn new(api_url: String, api_key: Option<String>) -> Self {
        Self {
            lifecycle: OpenSandboxClient::new(api_url, api_key),
            execd_clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create an ExecdClient for a sandbox.
    async fn get_execd(&self, id: &str) -> anyhow::Result<ExecdClient> {
        let clients = self.execd_clients.lock().await;
        if clients.contains_key(id) {
            drop(clients);
            let sandbox = self.lifecycle.get_sandbox(id).await?;
            let execd_url = sandbox
                .execd_url
                .unwrap_or_else(|| "http://localhost:44772".to_string());
            return Ok(ExecdClient::new(execd_url, sandbox.access_token));
        }
        drop(clients);

        let sandbox = self.lifecycle.get_sandbox(id).await?;
        let execd_url = sandbox
            .execd_url
            .unwrap_or_else(|| "http://localhost:44772".to_string());
        let execd = ExecdClient::new(execd_url.clone(), sandbox.access_token.clone());

        let mut clients = self.execd_clients.lock().await;
        clients.insert(
            id.to_string(),
            ExecdClient::new(execd_url, sandbox.access_token),
        );

        Ok(execd)
    }
}

#[async_trait]
impl ContainerRuntime for OpenSandboxRuntime {
    fn kind(&self) -> RuntimeKind {
        RuntimeKind::OpenSandbox
    }

    async fn is_available(&self) -> bool {
        self.lifecycle.health_check().await
    }

    async fn version(&self) -> anyhow::Result<String> {
        Ok("opensandbox".to_string())
    }

    async fn create(&self, config: &ContainerConfig) -> anyhow::Result<ContainerInfo> {
        let mut env_map = HashMap::new();
        for (k, v) in &config.env {
            env_map.insert(k.clone(), v.clone());
        }

        let mut labels = HashMap::new();
        labels.insert("vibecody".to_string(), "sandbox".to_string());

        let req = CreateSandboxRequest {
            image: Some(config.image.clone()),
            keep_alive_time_seconds: Some(config.timeout_secs),
            env: if env_map.is_empty() {
                None
            } else {
                Some(env_map)
            },
            labels: Some(labels),
        };

        let resp = self.lifecycle.create_sandbox(&req).await?;

        // Cache the execd client
        if let Some(ref url) = resp.execd_url {
            let mut clients = self.execd_clients.lock().await;
            clients.insert(
                resp.id.clone(),
                ExecdClient::new(url.clone(), resp.access_token.as_ref().cloned()),
            );
        }

        Ok(ContainerInfo {
            id: resp.id,
            name: String::new(),
            image: resp.image.unwrap_or_else(|| config.image.clone()),
            status: resp.status,
            created_at: resp.created_at.unwrap_or_default(),
            runtime: RuntimeKind::OpenSandbox,
        })
    }

    async fn stop(&self, id: &str) -> anyhow::Result<()> {
        self.lifecycle.delete_sandbox(id).await?;
        let mut clients = self.execd_clients.lock().await;
        clients.remove(id);
        Ok(())
    }

    async fn remove(&self, id: &str) -> anyhow::Result<()> {
        self.stop(id).await
    }

    async fn pause(&self, id: &str) -> anyhow::Result<()> {
        self.lifecycle.pause_sandbox(id).await
    }

    async fn resume(&self, id: &str) -> anyhow::Result<()> {
        self.lifecycle.resume_sandbox(id).await
    }

    async fn list(&self) -> anyhow::Result<Vec<ContainerInfo>> {
        let sandboxes = self.lifecycle.list_sandboxes(None).await?;
        Ok(sandboxes
            .into_iter()
            .map(|s| ContainerInfo {
                id: s.id,
                name: String::new(),
                image: s.image.unwrap_or_default(),
                status: s.status,
                created_at: s.created_at.unwrap_or_default(),
                runtime: RuntimeKind::OpenSandbox,
            })
            .collect())
    }

    async fn inspect(&self, id: &str) -> anyhow::Result<ContainerInfo> {
        let s = self.lifecycle.get_sandbox(id).await?;
        Ok(ContainerInfo {
            id: s.id,
            name: String::new(),
            image: s.image.unwrap_or_default(),
            status: s.status,
            created_at: s.created_at.unwrap_or_default(),
            runtime: RuntimeKind::OpenSandbox,
        })
    }

    async fn exec(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
    ) -> anyhow::Result<ExecResult> {
        let execd = self.get_execd(id).await?;
        let req = RunCommandRequest {
            command: command.to_string(),
            cwd: cwd.map(|s| s.to_string()),
            timeout: Some(300),
        };
        execd.run_command(&req).await
    }

    async fn exec_stream(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
        tx: mpsc::Sender<ExecStreamEvent>,
    ) -> anyhow::Result<()> {
        let execd = self.get_execd(id).await?;
        let req = RunCommandRequest {
            command: command.to_string(),
            cwd: cwd.map(|s| s.to_string()),
            timeout: Some(300),
        };
        execd.run_command_stream(&req, tx).await
    }

    async fn read_file(&self, id: &str, path: &str) -> anyhow::Result<String> {
        let execd = self.get_execd(id).await?;
        execd.download_file(path).await
    }

    async fn write_file(&self, id: &str, path: &str, content: &str) -> anyhow::Result<()> {
        let execd = self.get_execd(id).await?;
        execd.upload_file(path, content).await
    }

    async fn list_dir(&self, id: &str, path: &str) -> anyhow::Result<Vec<String>> {
        let execd = self.get_execd(id).await?;
        execd.list_dir(path).await
    }

    async fn logs(&self, id: &str, _tail: Option<u32>) -> anyhow::Result<String> {
        let result = self
            .exec(id, "cat /var/log/sandbox.log 2>/dev/null || echo '(no logs)'", None)
            .await?;
        Ok(result.stdout)
    }

    async fn metrics(&self, id: &str) -> anyhow::Result<ContainerMetrics> {
        let execd = self.get_execd(id).await?;
        execd.get_metrics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_stdout() {
        let body = "data: {\"type\":\"stdout\",\"text\":\"hello\\n\"}\ndata: {\"type\":\"stdout\",\"text\":\"world\\n\"}\ndata: {\"type\":\"result\",\"exit_code\":0}\n";
        let (stdout, stderr, exit_code) = parse_sse_events(body);
        assert_eq!(stdout, "hello\nworld\n");
        assert!(stderr.is_empty());
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn parse_sse_stderr() {
        let body = "data: {\"type\":\"stderr\",\"text\":\"error: not found\\n\"}\ndata: {\"type\":\"result\",\"exit_code\":1}\n";
        let (stdout, stderr, exit_code) = parse_sse_events(body);
        assert!(stdout.is_empty());
        assert_eq!(stderr, "error: not found\n");
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn parse_sse_mixed() {
        let body = "data: {\"type\":\"init\",\"id\":\"cmd-1\"}\ndata: {\"type\":\"stdout\",\"text\":\"ok\"}\ndata: {\"type\":\"stderr\",\"text\":\"warn\"}\ndata: {\"type\":\"execution_complete\",\"exit_code\":0}\n";
        let (stdout, stderr, exit_code) = parse_sse_events(body);
        assert_eq!(stdout, "ok");
        assert_eq!(stderr, "warn");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn parse_sse_empty() {
        let (stdout, stderr, exit_code) = parse_sse_events("");
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn parse_sse_invalid_json_ignored() {
        let body = "data: not json\ndata: {\"type\":\"stdout\",\"text\":\"ok\"}\n";
        let (stdout, _, _) = parse_sse_events(body);
        assert_eq!(stdout, "ok");
    }

    #[test]
    fn urlencoding_spaces() {
        assert_eq!(urlencoding_encode("/path/with space"), "/path/with%20space");
    }

    #[test]
    fn urlencoding_special_chars() {
        assert_eq!(urlencoding_encode("a?b&c"), "a%3Fb%26c");
    }

    #[test]
    fn create_sandbox_request_serialization() {
        let req = CreateSandboxRequest {
            image: Some("ubuntu:22.04".to_string()),
            keep_alive_time_seconds: Some(3600),
            env: None,
            labels: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"image\":\"ubuntu:22.04\""));
        assert!(json.contains("\"keepAliveTimeSeconds\":3600"));
    }

    #[test]
    fn execd_event_deserialization() {
        let json = r#"{"type":"stdout","text":"hello\n"}"#;
        let event: ExecdEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "stdout");
        assert_eq!(event.text, Some("hello\n".to_string()));
    }

    #[test]
    fn execd_event_with_exit_code() {
        let json = r#"{"type":"result","exit_code":42}"#;
        let event: ExecdEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "result");
        assert_eq!(event.exit_code, Some(42));
    }

    #[test]
    fn runtime_kind_is_opensandbox() {
        let rt = OpenSandboxRuntime::new("http://localhost:8080".to_string(), None);
        assert_eq!(rt.kind(), RuntimeKind::OpenSandbox);
    }
}
