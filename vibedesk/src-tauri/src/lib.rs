//! vibedesk — Tauri backend for the VibeDesk task-first companion app.
//!
//! VibeDesk does not re-implement any agent logic. It is a thin GUI over the
//! VibeCLI daemon, talking to it over HTTP/SSE via the commands in this crate
//! (the same pattern as vibeaichat). The daemon is the source of truth.

mod commands;
// Settings commands are shared with VibeAIChat — see
// crates/vibe-desktop-settings. Aliased so the `settings::…` paths below
// (and `generate_handler!`) read unchanged.
use vibe_desktop_settings::settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── A binary with no frontend in it ──────────────────────────────────
    // `cfg(dev)` means `generate_context!` embedded nothing and the window will
    // load `http://localhost:1422` instead. In a debug build that is the dev
    // server and correct; in a release build nothing is listening there and the
    // app opens on a blank white page. Nothing else — not a panic, not a log
    // line, not the window itself — says so, which is how a whole afternoon
    // goes into diagnosing an empty rectangle.
    #[cfg(all(dev, not(debug_assertions)))]
    eprintln!(
        "warning: this build has no frontend embedded and will load http://localhost:1422. \
         Build it with `npm run tauri:build` (or `make build-vibedesk`), or add `--features custom-protocol`."
    );

    // ── Fix PATH for macOS .app bundles ──────────────────────────────────
    // Finder/Launchpad gives apps a minimal PATH; source the user's shell for
    // the real one so a bundled VibeDesk can find `vibecli` on PATH.
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

            // One-time settings migration: carry global UI prefs forward from
            // the pre-rename `__vibex__` namespace so an upgraded install keeps
            // its theme and default provider/model. Idempotent + non-destructive.
            match settings::migrate_legacy_settings() {
                Ok(0) => {}
                Ok(n) => eprintln!("vibedesk: migrated {n} setting(s) from VibeX"),
                Err(e) => eprintln!("vibedesk: settings migration skipped ({e})"),
            }

            // Zero-config: autostart the VibeCLI daemon on launch so VibeDesk works
            // out of the box. Reuses an already-running daemon; only spawns one
            // if `/health` is unreachable. Fire-and-forget — the daemon-status
            // banner reflects the result as the daemon comes online.
            tauri::async_runtime::spawn(async {
                let port = commands::daemon_port();
                let state = commands::ensure_daemon_state(port).await;
                // Log the specific outcome, not a generic guess at the cause —
                // "is vibecli on PATH?" is wrong advice when the real problem
                // is a port conflict or a daemon that exited on startup.
                eprintln!("vibedesk: {}", state.user_message());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_daemon,
            commands::start_daemon,
            commands::list_daemon_models,
            commands::start_agent_session,
            commands::stream_agent,
            commands::cancel_agent_session,
            commands::get_job,
            commands::list_skills,
            commands::list_plugins,
            commands::plugin_catalog,
            commands::install_plugin,
            commands::set_plugin_policy,
            commands::uninstall_plugin,
            commands::list_connectors,
            commands::add_connector,
            commands::toggle_connector,
            commands::remove_connector,
            commands::probe_connector,
            commands::read_attachment,
            commands::run_command,
            commands::list_loops,
            commands::create_loop,
            commands::stop_loop,
            commands::open_browser,
            commands::get_skill,
            commands::stream_approvals,
            commands::respond_approval,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::merge_task,
            commands::list_tasks_by_state,
            commands::archive_task,
            commands::restore_task,
            commands::purge_task,
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
            // Voice input. The composer's mic button calls `transcribe_audio`
            // via `tauriTranscriber()` in packages/vibe-ui-shared; without
            // these registrations the button records and then silently fails.
            vibe_desktop_voice::transcribe_audio,
            vibe_desktop_voice::voice_status,
            // A WebSocket cannot set an Authorization header, so /ws/voice/duplex
            // takes ?token= and the frontend needs the effective token to build it.
            vibe_desktop_voice::daemon_token_effective,
            commands::daemon_port,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vibedesk");
}
