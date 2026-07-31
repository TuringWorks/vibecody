//! VibeDesk Tauri commands — a thin HTTP/SSE bridge to the VibeCLI daemon.
//! Adapted from vibeapp; adds `approval` + `reasoning` params (VX-107/108/111).
//! VibeDesk never re-implements daemon logic — the daemon is the source of truth.

use tauri::AppHandle;

/// Resolve the daemon bearer token. Prefers an explicit token from the caller,
/// then falls back to `~/.vibecli/daemon.token` (where `vibecli --serve` writes
/// it) and the `VIBECLI_TOKEN` env var. This keeps VibeDesk zero-config: the
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

/// Minimal home-dir lookup without pulling the `dirs` crate into vibedesk.
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

// ── Daemon autostart (zero-config) ──────────────────────────────────────────
//
// VibeDesk is a thin client over the VibeCLI daemon, so the daemon must be running
// for anything to work. Rather than make the user run `vibecli serve` by hand,
// VibeDesk starts it automatically on launch (see `lib.rs::run`'s setup hook) and
// exposes `start_daemon` so the UI can offer a retry if autostart failed. An
// already-running daemon is reused — it's shared infra for the mobile / watch /
// IDE clients, so we never start a second one.

/// Default daemon port — mirrors `DEFAULT_DAEMON_URL` (127.0.0.1:7878) in the
/// frontend. Override with the `VIBEDESK_DAEMON_PORT` env var.
pub fn daemon_port() -> u16 {
    std::env::var("VIBEDESK_DAEMON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7878)
}

/// True if the daemon answers `/health` on `127.0.0.1:port`. Any HTTP response
/// counts as "up" (mirrors `check_daemon`); only a transport error means down.
async fn daemon_health_ok(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_millis(800))
        .send()
        .await
        .is_ok()
}

/// Spawn the VibeCLI daemon, detached from VibeDesk's stdio so it outlives the
/// window (a persistent local service). Tries the `--serve` flag form first —
/// a `--`-prefixed token can only be parsed as a flag, never as an agent task,
/// so a binary that doesn't know it errors out fast and we fall back to the
/// `serve` subcommand form. Returns true once a child appears to stay alive.
/// Relies on `vibecli` being on PATH (lib.rs patches PATH for macOS bundles).
fn spawn_daemon(port: u16) -> bool {
    use std::process::{Command, Stdio};
    let port = port.to_string();
    // `--serve` first: safe because clap rejects an unknown `--serve` flag
    // (fast non-zero exit) rather than treating it as an agent prompt.
    let forms: [&[&str]; 2] = [&["--serve"], &["serve"]];
    for base in forms {
        let mut cmd = Command::new("vibecli");
        cmd.args(base)
            .arg("--port")
            .arg(&port)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match cmd.spawn() {
            Ok(mut child) => {
                // A wrong invocation form exits almost immediately; a real
                // daemon keeps running. Give it a beat, then decide.
                std::thread::sleep(std::time::Duration::from_millis(400));
                match child.try_wait() {
                    Ok(Some(status)) if !status.success() => continue, // try next form
                    _ => return true, // still running (daemon) or clean exit
                }
            }
            Err(_) => continue, // `vibecli` not found / not executable — try next
        }
    }
    false
}

/// Ensure the daemon is running on `port`: reuse it if `/health` already
/// answers, otherwise spawn it and poll until it binds (~10s). Returns whether
/// the daemon is up. Called on launch and by the `start_daemon` command.
pub async fn ensure_daemon_running(port: u16) -> bool {
    if daemon_health_ok(port).await {
        return true;
    }
    // Spawn off the async runtime — `spawn_daemon` blocks briefly on liveness.
    let started = tokio::task::spawn_blocking(move || spawn_daemon(port))
        .await
        .unwrap_or(false);
    if !started {
        return false;
    }
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if daemon_health_ok(port).await {
            return true;
        }
    }
    false
}

/// Start the VibeCLI daemon if it isn't already running (UI "retry" affordance
/// when autostart didn't take, e.g. `vibecli` wasn't on PATH at launch).
/// Returns "online" on success, or an error the banner can surface.
#[tauri::command]
pub async fn start_daemon(port: Option<u16>) -> Result<String, String> {
    let port = port.unwrap_or_else(daemon_port);
    if ensure_daemon_running(port).await {
        Ok("online".to_string())
    } else {
        Err(format!(
            "Could not start the VibeCLI daemon on port {}. Is `vibecli` installed and on your PATH? Try running `vibecli serve` in a terminal.",
            port
        ))
    }
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
    resume_session_id: Option<String>,
    workspace_root: Option<String>,
    token: Option<String>,
) -> Result<String, String> {
    let agent_url = format!("{}/agent", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "task": task,
        "provider": provider,
        "approval": approval.unwrap_or_else(|| "default".to_string()),
    });
    // Multi-project: run in the selected project (or the task's worktree) rather
    // than the daemon's own cwd. Omitted → the daemon falls back to its root.
    if let Some(root) = &workspace_root {
        if !root.is_empty() {
            body["workspace_root"] = serde_json::Value::String(root.clone());
        }
    }
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
    // VibeDesk resume: continue the selected chat's session instead of starting a
    // fresh one. The daemon reuses this id and seeds prior conversation.
    if let Some(rid) = &resume_session_id {
        if !rid.is_empty() {
            body["resume_session_id"] = serde_json::Value::String(rid.clone());
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

/// GET /api/tasks — list recent VibeDesk tasks (VX-112).
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
    title: Option<String>,
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
    if let Some(t) = &title {
        body["title"] = serde_json::Value::String(t.clone());
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

/// DELETE /api/tasks/:id — remove a task. When `remove_worktree` is true the
/// daemon also tears down the task's git worktree (VX bug-2 "delete chat").
#[tauri::command]
pub async fn delete_task(
    url: String,
    id: String,
    remove_worktree: Option<bool>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let task_url = format!(
        "{}/api/tasks/{}?remove_worktree={}",
        url.trim_end_matches('/'),
        id,
        remove_worktree.unwrap_or(false)
    );
    let client = reqwest::Client::new();
    let req = with_auth(client.delete(&task_url), token);
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

/// POST /api/tasks/:id/merge — merge the task's worktree branch back into the
/// project's branch, then remove the worktree + task on success. On conflict
/// the daemon aborts the merge and returns 409; the error string carries the
/// conflict detail so the UI can surface it (VX bug-2 "merge & delete").
#[tauri::command]
pub async fn merge_task(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let merge_url = format!("{}/api/tasks/{}/merge", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let req = with_auth(client.post(&merge_url), token);
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

/// GET /api/tasks?state=trashed|archived|all — list tasks filtered by lifecycle
/// state for the Trash / Archive recovery views (worktree-lifecycle slice 2).
#[tauri::command]
pub async fn list_tasks_by_state(
    url: String,
    state: String,
    token: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    // `state` is a fixed daemon-side enum (active|trashed|archived|all); the
    // daemon ignores unknown values (falls back to active), so no encoding needed.
    let tasks_url = format!("{}/api/tasks?state={}", url.trim_end_matches('/'), state);
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

/// POST /api/tasks/:id/archive — archive a task: its branch is kept, the reaper
/// frees the worktree directory, and restore re-creates it (worktree-lifecycle).
#[tauri::command]
pub async fn archive_task(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/api/tasks/{}/archive", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let req = with_auth(client.post(&u), token);
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

/// POST /api/tasks/:id/restore — bring a trashed/archived task back to Active,
/// re-materializing its worktree from the (possibly preserved) branch.
#[tauri::command]
pub async fn restore_task(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/api/tasks/{}/restore", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let req = with_auth(client.post(&u), token);
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

/// DELETE /api/tasks/:id?purge=true — permanently remove a task now. Routes
/// through the daemon's safe teardown (unmerged work is still preserved at
/// refs/trash/<id>), so it cannot silently discard work (worktree-lifecycle).
#[tauri::command]
pub async fn purge_task(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/api/tasks/{}?purge=true", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let req = with_auth(client.delete(&u), token);
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

/// GET /api/tasks/:id/history — fetch a finished chat's conversation
/// (reconstructed from the daemon's durable event log) so it can be re-rendered
/// in the center pane when the chat is selected (VX bug-3).
#[tauri::command]
pub async fn get_task_history(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let hist_url = format!("{}/api/tasks/{}/history", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let req = with_auth(client.get(&hist_url), token);
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
/// VibeDesk environment endpoints (git status/diff, files). `scope` is the
/// optional repo path those endpoints inspect — passed as `?path=` and encoded
/// by reqwest, so project paths with spaces work unchanged.
async fn daemon_get(
    url: String,
    path: &str,
    scope: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let full = format!("{}{}", url.trim_end_matches('/'), path);
    let client = reqwest::Client::new();
    let get = match scope.filter(|s| !s.is_empty()) {
        Some(p) => client.get(&full).query(&[("path", p)]),
        None => client.get(&full),
    };
    let resp = with_auth(get, token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// GET /api/vibedesk/git/status — branch + changed files (VX-109), scoped to
/// `path` (the active project / task worktree) when given.
#[tauri::command]
pub async fn git_status(
    url: String,
    path: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibedesk/git/status", path, token).await
}

/// GET /api/vibedesk/git/diff — working-tree diff for the Review action (VX-202).
#[tauri::command]
pub async fn git_diff(
    url: String,
    path: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibedesk/git/diff", path, token).await
}

/// GET /api/vibedesk/files — tracked file paths for the Files action (VX-110).
#[tauri::command]
pub async fn list_files(
    url: String,
    path: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibedesk/files", path, token).await
}

/// Normalise what a user typed into a URL safe to hand a webview.
///
/// Only `http`/`https` survive. A bare `example.com` gets `https://`; anything
/// with another scheme — `file:`, `javascript:`, `data:` — is rejected rather
/// than normalised, because those are exactly the ones that turn "open a page"
/// into "read the disk" or "run script in my window". Control characters are
/// refused too: a newline in a URL is how header/scheme smuggling starts.
fn normalize_browser_url(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("Enter a web address.".into());
    }
    if raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("That doesn't look like a web address.".into());
    }
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(raw.to_string());
    }
    // A scheme we don't allow — say so instead of silently prefixing https://
    // and opening something the user didn't ask for.
    if let Some(scheme_end) = raw.find(':') {
        let scheme = &lower[..scheme_end];
        if !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return Err(format!("Only http and https addresses can be opened (got \"{scheme}:\")."));
        }
    }
    Ok(format!("https://{raw}"))
}

/// Open a website in its own webview window.
///
/// A separate window rather than an in-app pane on purpose: the app's CSP is
/// `default-src 'self'`, so a remote page cannot be framed inside the shell,
/// and giving arbitrary web content a surface inside the app window would be
/// the wrong place to economise. The new window's label is not listed in
/// `capabilities/default.json` (`windows: ["main"]`), so the page it loads has
/// **no** access to any Tauri command — it is a browser tab, not part of the app.
#[tauri::command]
pub async fn open_browser(app: AppHandle, url: String) -> Result<String, String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let target = normalize_browser_url(&url)?;
    let parsed: tauri::Url = target
        .parse()
        .map_err(|e| format!("Could not parse \"{target}\": {e}"))?;

    // Unique label per window so several pages can be open at once; reusing a
    // label would silently fail once the first window exists.
    let label = format!(
        "browser-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed))
        .title(&target)
        .inner_size(1100.0, 800.0)
        .build()
        .map_err(|e| format!("Could not open a window: {e}"))?;
    Ok(target)
}

// ── Automations (hosted loops) ──────────────────────────────────────────────
//
// The daemon has a resident scheduler (`hosted_loop::scheduler_tick`) that runs
// recurring prompts and survives client disconnect, persisted to loops.json —
// but its only clients were the CLI REPL and raw HTTP, so VibeDesk's
// Automations nav item sat disabled. These three commands wire it up.

/// GET /v1/loops — hosted loop jobs the daemon is running.
#[tauri::command]
pub async fn list_loops(url: String, token: Option<String>) -> Result<serde_json::Value, String> {
    daemon_get(url, "/v1/loops", None, token).await
}

/// POST /v1/loops — create a hosted loop. `args` is the `/loop` argument
/// string: `<interval> <prompt>` (e.g. `5m run the tests`) or `auto <prompt>`.
/// The daemon parses it, so the accepted forms can't drift from the CLI's.
#[tauri::command]
pub async fn create_loop(
    url: String,
    args: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/v1/loops", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = with_auth(client.post(&u).json(&serde_json::json!({ "args": args })), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let b = resp.text().await.unwrap_or_default();
        return Err(format!("Daemon returned {}: {}", s, b));
    }
    // A bad `args` string comes back as 200 with an `error` field rather than a
    // status code — surface it as an error so the UI doesn't show a phantom
    // automation that was never created.
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    match body.get("error").and_then(|e| e.as_str()) {
        Some(msg) => Err(msg.to_string()),
        None => Ok(body),
    }
}

/// POST /v1/loops/:id/stop — stop a hosted loop. The scheduler skips terminal
/// jobs on its next tick.
#[tauri::command]
pub async fn stop_loop(
    url: String,
    id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/v1/loops/{}/stop", url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let resp = with_auth(client.post(&u), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// POST /api/vibedesk/exec — run one shell command in the scoped project.
/// Backs the Terminal quick-action. One-shot, not a PTY: the daemon bounds it
/// with a timeout and an output cap, and returns stdout/stderr/exit code.
#[tauri::command]
pub async fn run_command(
    url: String,
    command: String,
    path: Option<String>,
    timeout_ms: Option<u64>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/api/vibedesk/exec", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({ "command": command });
    if let Some(p) = path.filter(|s| !s.is_empty()) {
        body["path"] = serde_json::Value::String(p);
    }
    if let Some(t) = timeout_ms {
        body["timeout_ms"] = serde_json::Value::from(t);
    }
    let resp = with_auth(client.post(&u).json(&body), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let b = resp.text().await.unwrap_or_default();
        return Err(format!("Daemon returned {}: {}", status, b));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// Largest attachment we inline into a prompt. Past this the file is better
/// referenced by path (the agent has read tools) than pasted — and a multi-MB
/// paste mostly buys context-window exhaustion.
const MAX_ATTACHMENT_BYTES: usize = 256 * 1024;

/// Read a user-picked file for composer attachment.
///
/// Only ever called with a path the user chose in the native file dialog, so
/// there is no path-traversal surface to defend — but it still refuses what it
/// cannot usefully inline: binary content (invalid UTF-8) is rejected outright
/// rather than pasted as mojibake, and oversized text is truncated with the
/// truncation reported so the UI can say so instead of silently losing the tail.
#[tauri::command]
pub async fn read_attachment(path: String) -> Result<serde_json::Value, String> {
    let p = std::path::PathBuf::from(&path);
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let bytes = tokio::fs::read(&p)
        .await
        .map_err(|e| format!("Could not read {}: {}", name, e))?;
    let total = bytes.len();

    let (slice, truncated) = if total > MAX_ATTACHMENT_BYTES {
        (&bytes[..MAX_ATTACHMENT_BYTES], true)
    } else {
        (&bytes[..], false)
    };
    // Truncating at a byte offset can split a multi-byte char; take the valid
    // prefix rather than failing a file that is otherwise perfectly readable.
    let text = match std::str::from_utf8(slice) {
        Ok(t) => t.to_string(),
        Err(e) if truncated && e.valid_up_to() > 0 => {
            String::from_utf8_lossy(&slice[..e.valid_up_to()]).to_string()
        }
        Err(_) => {
            return Err(format!(
                "{} looks like a binary file — attach text, or reference it with @ so the agent can open it itself.",
                name
            ))
        }
    };

    Ok(serde_json::json!({
        "path": path,
        "name": name,
        "bytes": total,
        "text": text,
        "truncated": truncated,
    }))
}

/// GET /api/vibedesk/plugins — enabled plugin components for the scoped
/// workspace. Read-only inventory; enabling/disabling stays in the CLI and the
/// governance surface where it's audited.
#[tauri::command]
pub async fn list_plugins(
    url: String,
    path: Option<String>,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    daemon_get(url, "/api/vibedesk/plugins", path, token).await
}

/// POST /jobs/:id/cancel — stop an in-flight agent run. Backs the composer's
/// Stop button: a long or wrong-headed turn must be interruptible without
/// killing the app or waiting it out. The daemon marks the job cancelled and
/// closes its stream, so the UI's SSE forwarder terminates on its own.
#[tauri::command]
pub async fn cancel_agent_session(
    url: String,
    session_id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/jobs/{}/cancel", url.trim_end_matches('/'), session_id);
    let client = reqwest::Client::new();
    let resp = with_auth(client.post(&u), token)
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

/// GET /v1/skilllens/skills — the daemon's skill catalog (name, category,
/// summary, source). Backs the Skills browser, which was a disabled
/// "coming soon" nav item despite the catalog already being served.
#[tauri::command]
pub async fn list_skills(
    url: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    daemon_get(url, "/v1/skilllens/skills", None, token).await
}

/// GET /v1/skilllens/skills/:name — one skill plus its markdown body, for the
/// detail pane. The name comes from the catalog, so it is encoded as a path
/// segment rather than trusted verbatim.
#[tauri::command]
pub async fn get_skill(
    url: String,
    name: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let encoded: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect();
    daemon_get(url, &format!("/v1/skilllens/skills/{encoded}"), None, token).await
}

/// GET /jobs/:id — the daemon's record for a run, carrying `steps_completed`,
/// `tokens_used` and `cost_cents`. Backs the usage readout: without it a run is
/// opaque about what it is spending, which both Claude Desktop and Codex show.
#[tauri::command]
pub async fn get_job(
    url: String,
    session_id: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/jobs/{}", url.trim_end_matches('/'), session_id);
    let client = reqwest::Client::new();
    let resp = with_auth(client.get(&u), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

// ── Approval prompts (tainted-content gate) ─────────────────────────────────
//
// When the daemon runs with `--tainted-http-prompt`, an action carrying
// prompt-injection-tainted input blocks until a client approves it. VibeDesk's
// approval pill only ever chose a *policy*; there was no surface to answer an
// actual prompt, so a gated run would sit there with no explanation. These two
// commands bridge the daemon's queue to the UI. When the flag is off the SSE
// stream simply never emits — nothing to configure, nothing to break.

/// Tauri event carrying one pending approval prompt.
pub const APPROVAL_EVENT: &str = "approval://pending";

/// GET /v1/tainted/pending — forward the daemon's SSE prompt queue. The daemon
/// re-emits a full snapshot on every change, so the UI de-dupes by `request_id`.
#[tauri::command]
pub async fn stream_approvals(
    app: AppHandle,
    url: String,
    token: Option<String>,
) -> Result<(), String> {
    use tauri::Emitter;

    let u = format!("{}/v1/tainted/pending", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let res = with_auth(client.get(&u).header("Accept", "text/event-stream"), token)
        .send()
        .await
        .map_err(|e| format!("Cannot connect to approval stream: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Approval stream returned {}", res.status()));
    }

    tokio::spawn(async move {
        let mut buf = String::new();
        let mut response = res;
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(c)) => c,
                _ => break,
            };
            let Ok(text) = std::str::from_utf8(&chunk) else {
                continue;
            };
            buf.push_str(text);
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf = buf[nl + 1..].to_string();
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(ev) = serde_json::from_str::<serde_json::Value>(data) {
                        let _ = app.emit(APPROVAL_EVENT, ev);
                    }
                }
            }
        }
    });

    Ok(())
}

/// POST /v1/tainted/respond — approve or deny a pending prompt. A 404 means the
/// prompt already timed out or was answered elsewhere; the UI drops it either
/// way, so that is reported back rather than treated as a hard failure.
#[tauri::command]
pub async fn respond_approval(
    url: String,
    request_id: String,
    approve: bool,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let u = format!("{}/v1/tainted/respond", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "request_id": request_id, "approve": approve });
    let resp = with_auth(client.post(&u).json(&body), token)
        .send()
        .await
        .map_err(|e| format!("Cannot reach daemon: {}", e))?;
    let status = resp.status();
    let parsed: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
        return Err(format!("Daemon returned {}: {}", status, parsed));
    }
    Ok(parsed)
}

/// Tauri event name carrying one session's forwarded agent events. Scoping the
/// channel by session id is what lets several chats run at once: a global
/// `agent:chunk` channel delivered every run's tokens to every listener, so two
/// concurrent runs interleaved into whichever bubble was on screen. One channel
/// per session means a listener only ever sees its own run.
pub fn session_event_name(session_id: &str) -> String {
    format!("agent://{}", session_id)
}

/// Connect to the daemon SSE stream and forward its events to the frontend on
/// this session's own channel, as `{ kind, ... }` payloads:
///   `user`/`chunk`/`system`/`error` → `text`, `step` → `tool` + `summary`,
///   `complete` → this turn ended, `closed` → the daemon closed the stream.
///
/// `since_seq` replays the durable log from that sequence before going live, so
/// re-opening a chat whose run is still in flight catches up instead of showing
/// a gap. A replay of a multi-turn chat contains one `complete` per finished
/// turn, so `complete` must NOT end the forwarder — doing that truncated the
/// replay at the first turn. We read until the daemon closes the stream (which
/// it does once a run finishes) and emit `closed` then.
#[tauri::command]
pub async fn stream_agent(
    app: AppHandle,
    url: String,
    session_id: String,
    since_seq: Option<u64>,
    token: Option<String>,
) -> Result<(), String> {
    use tauri::Emitter;

    let base = format!("{}/stream/{}", url.trim_end_matches('/'), session_id);
    let client = reqwest::Client::new();
    let get = client.get(&base).header("Accept", "text/event-stream");
    let get = match since_seq {
        Some(n) => get.query(&[("since_seq", n)]),
        None => get,
    };
    let res = with_auth(get, token)
        .send()
        .await
        .map_err(|e| format!("Cannot connect to stream: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Stream returned {}", res.status()));
    }

    let channel = session_event_name(&session_id);
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
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let Ok(ev) = serde_json::from_str::<serde_json::Value>(data) else {
                    continue;
                };
                let content = ev["content"].as_str().unwrap_or("").to_string();
                match ev["type"].as_str() {
                    // `user` appears on replay (the durable log is a full
                    // transcript); the live path never sees it, because the
                    // daemon publishes it before this forwarder connects.
                    Some(kind @ ("user" | "chunk" | "system")) => {
                        let _ = app
                            .emit(&channel, serde_json::json!({ "kind": kind, "text": content }));
                    }
                    Some("step") => {
                        // Tool-use step — forward the tool name + summary so the
                        // UI can render a structured ToolUseBlock.
                        let tool = ev["tool_name"].as_str().unwrap_or("tool").to_string();
                        let _ = app.emit(
                            &channel,
                            serde_json::json!({ "kind": "step", "tool": tool, "summary": content }),
                        );
                    }
                    Some("complete") => {
                        let _ = app.emit(&channel, serde_json::json!({ "kind": "complete" }));
                    }
                    Some("error") => {
                        let text = if content.is_empty() {
                            "unknown error".to_string()
                        } else {
                            content
                        };
                        let _ =
                            app.emit(&channel, serde_json::json!({ "kind": "error", "text": text }));
                    }
                    _ => {}
                }
            }
        }
        // Stream closed — the run is over (or the daemon went away). The UI
        // unsubscribes on this, so it never sticks on "running" forever.
        let _ = app.emit(&channel, serde_json::json!({ "kind": "closed" }));
    });

    Ok(())
}
