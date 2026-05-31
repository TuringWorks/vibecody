//! VibeX Tauri commands — a thin HTTP/SSE bridge to the VibeCLI daemon.
//! Adapted from vibeapp; adds `approval` + `reasoning` params (VX-107/108/111).
//! VibeX never re-implements daemon logic — the daemon is the source of truth.

use tauri::AppHandle;

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
    let mut req = client.post(&agent_url).json(&body);
    if let Some(t) = &token {
        if !t.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
    }
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
    let mut req = client.get(&stream_url).header("Accept", "text/event-stream");
    if let Some(t) = &token {
        if !t.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
    }
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
                            Some("complete") => {
                                let _ = app.emit("agent:complete", ());
                                return;
                            }
                            Some("error") => {
                                let msg =
                                    ev["content"].as_str().unwrap_or("unknown error").to_string();
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
