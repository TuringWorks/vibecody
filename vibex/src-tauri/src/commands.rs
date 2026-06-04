//! VibeX Tauri commands — a thin HTTP/SSE bridge to the VibeCLI daemon.
//! Adapted from vibeapp; adds `approval` + `reasoning` params (VX-107/108/111).
//! VibeX never re-implements daemon logic — the daemon is the source of truth.

use tauri::AppHandle;

/// Resolve the daemon bearer token. Prefers an explicit token from the caller,
/// then falls back to `~/.vibecli/daemon.token` (where `vibecli --serve` writes
/// it) and the `VIBECLI_TOKEN` env var. This keeps VibeX zero-config: the
/// frontend never has to know the token — the local daemon's token file is the
/// source of truth. Returns `None` if no token is found (the daemon may be
/// running without auth).
fn resolve_token(explicit: Option<String>) -> Option<String> {
    if let Some(t) = explicit {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Ok(t) = std::env::var("VIBECLI_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    let path = dirs_home()?.join(".vibecli").join("daemon.token");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Minimal home-dir lookup without pulling the `dirs` crate into vibex.
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

/// Apply bearer auth to a request using the resolved token (if any).
fn with_auth(req: reqwest::RequestBuilder, token: Option<String>) -> reqwest::RequestBuilder {
    match resolve_token(token) {
        Some(t) => req.header("Authorization", format!("Bearer {}", t)),
        None => req,
    }
}

/// Ping the vibecli daemon `/health` endpoint; return "online" or an error.
#[tauri::command]
pub async fn check_daemon(url: String) -> Result<String, String> {
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    reqwest::get(&health_url)
        .await
        .map_err(|e| format!("Cannot reach daemon at {}: {}", url, e))?;
    Ok("online".to_string())
}

/// Fetch available models from the daemon's `/models` endpoint.
#[tauri::command]
pub async fn list_daemon_models(url: String) -> Result<Vec<serde_json::Value>, String> {
    let models_url = format!("{}/models", url.trim_end_matches('/'));
    let resp = reqwest::get(&models_url)
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body["models"].as_array().cloned().unwrap_or_default())
}

/// POST a task to the daemon `/agent` endpoint; return the session_id.
/// `approval` maps to the composer approval pill (VX-107); `reasoning` to the
/// reasoning-effort pill (VX-108) and is plumbed daemon-side in VX-111.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_agent_session(
    url: String,
    task: String,
    provider: String,
    model: Option<String>,
    approval: Option<String>,
    reasoning: Option<String>,
    token: Option<String>,
) -> Result<String, String> {
    let agent_url = format!("{}/agent", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "task": task,
        "provider": provider,
        "approval": approval.unwrap_or_else(|| "default".to_string()),
    });
    if let Some(m) = &model {
        if !m.is_empty() {
            body["model"] = serde_json::Value::String(m.clone());
        }
    }
    if let Some(r) = &reasoning {
        if !r.is_empty() {
            body["reasoning"] = serde_json::Value::String(r.clone());
        }
    }
    let req = with_auth(client.post(&agent_url).json(&body), token);
    let res = req
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Daemon returned {}: {}", status, body));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;

    body["session_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No session_id in response".to_string())
}

/// GET /api/tasks — list recent VibeX tasks (VX-112).
#[tauri::command]
pub async fn list_tasks(
    url: String,
    token: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let tasks_url = format!("{}/api/tasks", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let req = with_auth(client.get(&tasks_url), token);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body.as_array().cloned().unwrap_or_default())
}

/// POST /api/tasks — create a task (and its worktree). Returns the task row
/// (VX-112 + VX-113). The frontend then starts an agent and PATCHes the
/// returned session_id back via `update_task`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_task(
    url: String,
    title: String,
    provider: Option<String>,
    model: Option<String>,
    project_path: Option<String>,
    create_worktree: Option<bool>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let tasks_url = format!("{}/api/tasks", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "title": title,
        "create_worktree": create_worktree.unwrap_or(true),
    });
    if let Some(p) = &provider {
        if !p.is_empty() {
            body["provider"] = serde_json::Value::String(p.clone());
        }
    }
    if let Some(m) = &model {
        if !m.is_empty() {
            body["model"] = serde_json::Value::String(m.clone());
        }
    }
    if let Some(pp) = &project_path {
        if !pp.is_empty() {
            body["project_path"] = serde_json::Value::String(pp.clone());
        }
    }
    let req = with_auth(client.post(&tasks_url).json(&body), token);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let b = resp.text().await.unwrap_or_default();
        return Err(format!("Daemon returned {}: {}", status, b));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// PATCH /api/tasks/:id — update task status and/or link a session (VX-112).
#[tauri::command]
pub async fn update_task(
    url: String,
    id: String,
    status: Option<String>,
    session_id: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let task_url = format!("{}/api/tasks/{}", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({});
    if let Some(s) = &status {
        body["status"] = serde_json::Value::String(s.clone());
    }
    if let Some(sid) = &session_id {
        body["session_id"] = serde_json::Value::String(sid.clone());
    }
    let req = with_auth(client.patch(&task_url).json(&body), token);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// Generic authed GET against the daemon, returning parsed JSON. Shared by the
/// VibeX environment endpoints (git status/diff, files).
async fn daemon_get(
    url: String,
    path: &str,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let full = format!("{}{}", url.trim_end_matches('/'), path);
    let client = reqwest::Client::new();
    let req = with_auth(client.get(&full), token);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// GET /api/vibex/git/status — branch + changed files (VX-109).
#[tauri::command]
pub async fn git_status(url: String, token: Option<String>) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibex/git/status", token).await
}

/// GET /api/vibex/git/diff — working-tree diff for the Review action (VX-202).
#[tauri::command]
pub async fn git_diff(url: String, token: Option<String>) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibex/git/diff", token).await
}

/// GET /api/vibex/files — tracked file paths for the Files action (VX-110).
#[tauri::command]
pub async fn list_files(url: String, token: Option<String>) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibex/files", token).await
}

/// Connect to the daemon SSE stream and forward events to the frontend as
/// `agent:chunk` / `agent:complete` / `agent:error` events.
#[tauri::command]
pub async fn stream_agent(
    app: AppHandle,
    url: String,
    session_id: String,
    token: Option<String>,
) -> Result<(), String> {
    use tauri::Emitter;

    let stream_url = format!("{}/stream/{}", url.trim_end_matches('/'), session_id);
    let client = reqwest::Client::new();
    let req = with_auth(
        client
            .get(&stream_url)
            .header("Accept", "text/event-stream"),
        token,
    );
    let res = req
        .send()
        .await
        .map_err(|e| format!("Cannot connect to stream: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Stream returned {}", res.status()));
    }

    tokio::spawn(async move {
        let mut buf = String::new();
        let mut response = res;

        loop {
            let chunk = match response.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(_) => break,
            };
            let text = match std::str::from_utf8(&chunk) {
                Ok(t) => t,
                Err(_) => continue,
            };
            buf.push_str(text);

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf = buf[nl + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(ev) = serde_json::from_str::<serde_json::Value>(data) {
                        match ev["type"].as_str() {
                            Some("chunk") => {
                                if let Some(t) = ev["content"].as_str() {
                                    let _ = app.emit("agent:chunk", t.to_string());
                                }
                            }
                            Some("system") => {
                                if let Some(t) = ev["content"].as_str() {
                                    let _ = app.emit("agent:system", t.to_string());
                                }
                            }
                            Some("step") => {
                                // Tool-use step — forward the tool name + summary
                                // so the UI can render a structured ToolUseBlock.
                                let tool = ev["tool_name"].as_str().unwrap_or("tool").to_string();
                                let summary = ev["content"].as_str().unwrap_or("").to_string();
                                let _ = app.emit(
                                    "agent:step",
                                    serde_json::json!({ "tool": tool, "summary": summary }),
                                );
                            }
                            Some("complete") => {
                                let _ = app.emit("agent:complete", ());
                                return;
                            }
                            Some("error") => {
                                let msg = ev["content"]
                                    .as_str()
                                    .unwrap_or("unknown error")
                                    .to_string();
                                let _ = app.emit("agent:error", msg);
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        let _ = app.emit("agent:complete", ());
    });

    Ok(())
}
