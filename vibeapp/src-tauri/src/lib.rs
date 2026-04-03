//! vibeapp — Tauri backend for the floating VibeCLI AI window.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Fix PATH for macOS .app bundles ──────────────────────────────────
    // Finder/Launchpad gives apps a minimal PATH; source user's shell for the real one.
    #[cfg(target_os = "macos")]
    {
        if let Ok(shell) = std::env::var("SHELL").or_else(|_| Ok::<String, std::env::VarError>("/bin/zsh".to_string())) {
            if let Ok(output) = std::process::Command::new(&shell)
                .args(["-l", "-c", "echo __PATH_START__${PATH}__PATH_END__"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let (Some(start), Some(end)) = (stdout.find("__PATH_START__"), stdout.find("__PATH_END__")) {
                    let shell_path = &stdout[start + 14..end];
                    let current = std::env::var("PATH").unwrap_or_default();
                    let merged = if current.is_empty() {
                        shell_path.to_string()
                    } else {
                        format!("{shell_path}:{current}")
                    };
                    std::env::set_var("PATH", &merged);
                }
            }
        }
    }

    tauri::Builder::default()
        .setup(|app| {
            // Set the window icon so it shows in dock/taskbar (dev + production)
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes: &[u8] = include_bytes!("../icons/128x128.png");
                let icon = tauri::image::Image::from_bytes(icon_bytes)
                    .expect("Failed to load app icon");
                let _ = window.set_icon(icon);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_always_on_top,
            commands::start_drag,
            commands::hide_window,
            commands::show_window,
            commands::check_daemon,
            commands::list_daemon_models,
            commands::start_agent_session,
            commands::stream_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vibeapp");
}
