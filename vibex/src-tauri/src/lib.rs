//! vibex — Tauri backend for the VibeX task-first companion app.
//!
//! VibeX does not re-implement any agent logic. It is a thin GUI over the
//! VibeCLI daemon, talking to it over HTTP/SSE via the commands in this crate
//! (the same pattern as vibeapp). The daemon is the source of truth.

mod commands;
mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Fix PATH for macOS .app bundles ──────────────────────────────────
    // Finder/Launchpad gives apps a minimal PATH; source the user's shell for
    // the real one so a bundled VibeX can find `vibecli` on PATH.
    #[cfg(target_os = "macos")]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        if let Ok(output) = std::process::Command::new(&shell)
            .args(["-l", "-c", "echo __PATH_START__${PATH}__PATH_END__"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let (Some(start), Some(end)) =
                (stdout.find("__PATH_START__"), stdout.find("__PATH_END__"))
            {
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

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))
                {
                    let _ = window.set_icon(icon);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_daemon,
            commands::list_daemon_models,
            commands::start_agent_session,
            commands::stream_agent,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::merge_task,
            commands::get_task_history,
            commands::git_status,
            commands::git_diff,
            commands::list_files,
            settings::provider_key_set,
            settings::provider_key_has,
            settings::provider_key_list,
            settings::provider_key_delete,
            settings::provider_config_set,
            settings::provider_config_get_all,
            settings::setting_set,
            settings::setting_get,
            settings::setting_get_all,
            settings::oauth_client_set,
            settings::oauth_client_has,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vibex");
}
