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
    effort: Option<String>,
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
    // C5: forward the per-request effort tier as the daemon's `reasoning` field,
    // which resolves to the unified Effort (Claude/Gemini thinking budget,
    // OpenAI reasoning_effort). Omitted/empty → daemon/provider default.
    if let Some(e) = &effort {
        if !e.is_empty() {
            body["reasoning"] = serde_json::Value::String(e.clone());
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
                Ok(None) => break, // stream ended
                Err(_) => break,   // read error
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
                                if let Some(t) = ev["content"].as_str() {
                                    let _ = app.emit("agent:chunk", t.to_string());
                                }
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
        // Stream ended without explicit complete
        let _ = app.emit("agent:complete", ());
    });

    Ok(())
}

// ── SkillForge — Tauri commands proxying to the vibecli daemon (G7) ──────────
//
// vibeapp is a lightweight companion overlay (no AiMlComposite, no model
// toolbar), so it doesn't ship the full `SkillForgePanel.tsx` UI — that lives
// in VibeUI. These 10 commands register the SkillForge surface so it is
// *reachable* from vibeapp via `invoke()` for any future UI, with the same
// daemon-proxy + STRICT semantics as VibeUI: thin HTTP proxies to
// `http://localhost:7878/v1/skilllens/*` + `/v1/skillopt/*`, authenticated
// with the daemon bearer token at `~/.vibecli/daemon.token`. The daemon stays
// the single source of truth. Each LLM-calling command takes `provider` +
// `model` and forwards them in the request body — STRICT provider-agnostic,
// never a hard-coded default (see CLAUDE.md → Provider-Agnostic Panels).

const SKILLFORGE_DAEMON_BASE: &str = "http://localhost:7878";

/// Read the daemon bearer token from `~/.vibecli/daemon.token`. Uses
/// `$HOME` directly (vibeapp doesn't depend on the `dirs` crate) —
/// equivalent to `dirs::home_dir()` on macOS/Linux.
fn daemon_bearer_token() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let token = std::fs::read_to_string(format!("{home}/.vibecli/daemon.token"))
        .unwrap_or_default()
        .trim()
        .to_string();
    if token.is_empty() {
        Err(
            "daemon not running or no token at ~/.vibecli/daemon.token — start `vibecli serve`"
                .to_string(),
        )
    } else {
        Ok(token)
    }
}

async fn skillforge_daemon_get(path: &str) -> Result<serde_json::Value, String> {
    let token = daemon_bearer_token()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{SKILLFORGE_DAEMON_BASE}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if status.is_success() {
        Ok(json)
    } else {
        Err(format!("daemon {path} → {}: {json}", status.as_u16()))
    }
}

async fn skillforge_daemon_post(
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let token = daemon_bearer_token()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{SKILLFORGE_DAEMON_BASE}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if status.is_success() {
        Ok(json)
    } else {
        Err(format!("daemon {path} → {}: {json}", status.as_u16()))
    }
}

/// `GET /v1/skilllens/skills` — catalog list with cached scores. No LLM.
#[tauri::command]
pub async fn skilllens_list_skills() -> Result<serde_json::Value, String> {
    skillforge_daemon_get("/v1/skilllens/skills").await
}

/// `GET /v1/skilllens/skills/:name` — one skill + body + cached report.
#[tauri::command]
pub async fn skilllens_get_skill(name: String) -> Result<serde_json::Value, String> {
    let path = format!("/v1/skilllens/skills/{name}");
    skillforge_daemon_get(&path).await
}

/// `POST /v1/skilllens/refresh` — re-read the skills dir, reset the cache.
#[tauri::command]
pub async fn skilllens_refresh() -> Result<serde_json::Value, String> {
    skillforge_daemon_post("/v1/skilllens/refresh", &serde_json::json!({})).await
}

/// `POST /v1/skilllens/convert` — raw agent runs (JSONL) → ExperiencePool.
#[tauri::command]
pub async fn skilllens_convert(runs: String) -> Result<serde_json::Value, String> {
    skillforge_daemon_post("/v1/skilllens/convert", &serde_json::json!({ "runs": runs })).await
}

/// `POST /v1/skilllens/extract` — distil candidate skills from a pool. LLM.
#[tauri::command]
pub async fn skilllens_extract(
    pool: String,
    method: String,
    provider: String,
    model: String,
) -> Result<serde_json::Value, String> {
    skillforge_daemon_post(
        "/v1/skilllens/extract",
        &serde_json::json!({ "pool": pool, "method": method, "provider": provider, "model": model }),
    )
    .await
}

/// `POST /v1/skilllens/score` — measure a skill (target_evolvability + coverage). LLM.
#[tauri::command]
pub async fn skilllens_score(
    skill: String,
    tasks: Option<String>,
    provider: String,
    model: String,
) -> Result<serde_json::Value, String> {
    skillforge_daemon_post(
        "/v1/skilllens/score",
        &serde_json::json!({ "skill": skill, "tasks": tasks, "provider": provider, "model": model }),
    )
    .await
}

/// `POST /v1/skillopt/train` — spawn a training run, return `{job_id}`. LLM.
#[tauri::command(rename_all = "camelCase")]
pub async fn skillopt_train(
    skill: String,
    env_kind: String,
    env_tasks: Option<String>,
    config: Option<serde_json::Value>,
    provider: String,
    model: String,
) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "skill": skill,
        "env": { "kind": env_kind, "tasks": env_tasks },
        "config": config.unwrap_or_else(|| serde_json::json!({})),
        "provider": provider,
        "model": model,
    });
    skillforge_daemon_post("/v1/skillopt/train", &body).await
}

/// `GET /v1/skillopt/status/:job` — Running | Done | Failed | Cancelled.
#[tauri::command(rename_all = "camelCase")]
pub async fn skillopt_status(job_id: String) -> Result<serde_json::Value, String> {
    let path = format!("/v1/skillopt/status/{job_id}");
    skillforge_daemon_get(&path).await
}

/// `POST /v1/skillopt/cancel/:job` — best-effort cancel.
#[tauri::command(rename_all = "camelCase")]
pub async fn skillopt_cancel(job_id: String) -> Result<serde_json::Value, String> {
    let path = format!("/v1/skillopt/cancel/{job_id}");
    skillforge_daemon_post(&path, &serde_json::json!({})).await
}

/// `POST /v1/skillopt/promote` — write `*.opt.md` next to the shipped skill.
#[tauri::command]
pub async fn skillopt_promote(skill: String, content: String) -> Result<serde_json::Value, String> {
    skillforge_daemon_post(
        "/v1/skillopt/promote",
        &serde_json::json!({ "skill": skill, "content": content }),
    )
    .await
}
