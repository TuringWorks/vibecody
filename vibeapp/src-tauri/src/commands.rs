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
