//! Tauri commands for the vibeapp floating window.

use tauri::{AppHandle, Manager};

/// Toggle "always on top" for the main window.
#[tauri::command]
pub async fn set_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?
        .set_always_on_top(always_on_top)
        .map_err(|e| e.to_string())
}

/// Start dragging the frameless window.
#[tauri::command]
pub async fn start_drag(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?
        .start_dragging()
        .map_err(|e| e.to_string())
}

/// Hide the main window (send to tray).
#[tauri::command]
pub async fn hide_window(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?
        .hide()
        .map_err(|e| e.to_string())
}

/// Show the main window (restore from tray).
#[tauri::command]
pub async fn show_window(app: AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())
}

/// Ping the vibecli daemon and return "online" or an error message.
#[tauri::command]
pub async fn check_daemon(url: String) -> Result<String, String> {
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    reqwest::get(&health_url)
        .await
        .map_err(|e| format!("Cannot reach daemon at {}: {}", url, e))?;
    Ok("online".to_string())
}

/// Fetch available models from the daemon's /models endpoint.
#[tauri::command]
pub async fn list_daemon_models(url: String) -> Result<Vec<serde_json::Value>, String> {
    let models_url = format!("{}/models", url.trim_end_matches('/'));
    let resp = reqwest::get(&models_url)
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body["models"].as_array().cloned().unwrap_or_default())
}

/// POST to daemon /agent endpoint — returns session_id.
/// Proxied through Tauri to bypass CORS.
#[tauri::command]
pub async fn start_agent_session(
    url: String,
    task: String,
    provider: String,
    model: Option<String>,
    token: Option<String>,
) -> Result<String, String> {
    let agent_url = format!("{}/agent", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "task": task,
        "provider": provider,
        "approval": "full-auto",
    });
    if let Some(m) = &model {
        if !m.is_empty() {
            body["model"] = serde_json::Value::String(m.clone());
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

/// Connect to daemon SSE stream and forward events to the frontend.
/// Emits "agent:chunk", "agent:complete", and "agent:error" events.
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
    let mut req = client
        .get(&stream_url)
        .header("Accept", "text/event-stream");
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

    // Read SSE stream using reqwest's chunk() API (no extra crates needed)
    tokio::spawn(async move {
        let mut buf = String::new();
        let mut response = res;

        loop {
            let chunk = match response.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,    // stream ended
                Err(_) => break,      // read error
            };
            let text = match std::str::from_utf8(&chunk) {
                Ok(t) => t,
                Err(_) => continue,
            };
            buf.push_str(text);

            // Process complete lines
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
                                if let Some(t) = ev["text"].as_str() {
                                    let _ = app.emit("agent:chunk", t.to_string());
                                }
                            }
                            Some("complete") => {
                                let _ = app.emit("agent:complete", ());
                                return;
                            }
                            Some("error") => {
                                let msg = ev["message"]
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
        // Stream ended without explicit complete
        let _ = app.emit("agent:complete", ());
    });

    Ok(())
}
