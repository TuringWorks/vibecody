//! vibeaichat — Tauri backend for the floating VibeCLI AI window.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── A binary with no frontend in it ──────────────────────────────────
    // `cfg(dev)` means `generate_context!` embedded nothing and the window will
    // load `http://localhost:1421` instead. In a debug build that is the dev
    // server and correct; in a release build nothing is listening there and the
    // app opens on a blank white page. Nothing else — not a panic, not a log
    // line, not the window itself — says so, which is how a whole afternoon
    // goes into diagnosing an empty rectangle.
    #[cfg(all(dev, not(debug_assertions)))]
    eprintln!(
        "warning: this build has no frontend embedded and will load http://localhost:1421. \
         Build it with `npm run tauri:build` (or `make build-vibeaichat`), or add `--features custom-protocol`."
    );

    // ── Fix PATH for macOS .app bundles ──────────────────────────────────
    // Finder/Launchpad gives apps a minimal PATH; source user's shell for the real one.
    #[cfg(target_os = "macos")]
    {
        if let Ok(shell) = std::env::var("SHELL")
            .or_else(|_| Ok::<String, std::env::VarError>("/bin/zsh".to_string()))
        {
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
    }

    tauri::Builder::default()
        .setup(|app| {
            // Set the window icon so it shows in dock/taskbar (dev + production)
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes: &[u8] = include_bytes!("../icons/128x128.png");
                let icon =
                    tauri::image::Image::from_bytes(icon_bytes).expect("Failed to load app icon");
                let _ = window.set_icon(icon);
            }

            // The tray icon is declared in tauri.conf.json, so it appears — but
            // Tauri does not give it any behaviour. Without the handlers below,
            // "send to tray" hid the window and nothing brought it back: the
            // icon was there, and clicking it did nothing. Hiding a window with
            // no way to restore it is losing it.
            wire_tray(app.handle())?;

            // Zero-config: autostart the VibeCLI daemon on launch so this shell works
            // out of the box, exactly as VibeCoder and VibeDesk do. Every daemon
            // route but a handful needs a bearer token, and that token only exists
            // once a daemon has started and written it — so a shell that never
            // starts one 401s on everything with nothing on screen explaining why.
            //
            // Reuses an already-running daemon (identity-checked via `/health`), so
            // launching alongside VibeCoder does not spawn a second one.
            // Fire-and-forget: the daemon-status banner reflects the outcome.
            tauri::async_runtime::spawn(async {
                let port = commands::daemon_port();
                let state = commands::ensure_daemon_state(port).await;
                // Log the specific outcome. Each failure is a distinct state with
                // its own remedy — a port conflict and a missing binary need
                // opposite advice.
                eprintln!("vibeaichat: {}", state.user_message());
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_always_on_top,
            commands::start_drag,
            commands::hide_window,
            commands::show_window,
            commands::quit_app,
            commands::check_daemon,
            commands::start_daemon,
            commands::list_daemon_models,
            commands::start_agent_session,
            commands::stream_agent,
            // Settings — Providers / Appearance / Account. Shared with VibeDesk
            // (crates/vibe-desktop-settings) so key storage cannot diverge
            // between the two shells. The UI is packages/vibe-ui-shared; without
            // these registrations it renders and silently does nothing.
            vibe_desktop_settings::settings::provider_key_set,
            vibe_desktop_settings::settings::provider_key_has,
            vibe_desktop_settings::settings::provider_key_list,
            vibe_desktop_settings::settings::provider_key_delete,
            vibe_desktop_settings::settings::provider_config_set,
            vibe_desktop_settings::settings::provider_config_get_all,
            vibe_desktop_settings::settings::setting_set,
            vibe_desktop_settings::settings::setting_get,
            vibe_desktop_settings::settings::setting_get_all,
            vibe_desktop_settings::settings::oauth_client_set,
            vibe_desktop_settings::settings::oauth_client_has,
            // Remote-daemon bearer token, encrypted in the same store. Without
            // these the settings field saves nowhere and every request to a
            // remote daemon 401s.
            vibe_desktop_settings::settings::daemon_token_set,
            vibe_desktop_settings::settings::daemon_token_get,
            // Voice input. The composer's mic button calls `transcribe_audio`
            // via `tauriTranscriber()` in packages/vibe-ui-shared; without
            // these registrations the button records and then silently fails.
            vibe_desktop_voice::transcribe_audio,
            vibe_desktop_voice::voice_status,
            // A WebSocket cannot set an Authorization header, so /ws/voice/duplex
            // takes ?token= and the frontend needs the effective token to build it.
            vibe_desktop_voice::daemon_token_effective,
            commands::daemon_port,
            // SkillForge — 10 daemon-proxy commands (G7). vibeaichat's
            // bespoke UI doesn't render the panel (that lives in VibeCoder),
            // but the surface is registered so SkillForge is reachable
            // from vibeaichat via `invoke()`.
            commands::skilllens_list_skills,
            commands::skilllens_get_skill,
            commands::skilllens_refresh,
            commands::skilllens_convert,
            commands::skilllens_extract,
            commands::skilllens_score,
            commands::skillopt_train,
            commands::skillopt_status,
            commands::skillopt_cancel,
            commands::skillopt_promote,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vibeaichat");
}

/// Give the tray icon its behaviour: left-click toggles the window, and a menu
/// offers Show / Quit for anyone who right-clicks or whose platform routes the
/// click to a menu instead of an event.
fn wire_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
    use tauri::Manager;

    let show = MenuItem::with_id(app, "show", "Show VibeAIChat", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    // Declared in tauri.conf.json without an explicit `id`, which Tauri names
    // "main". If that ever changes this returns None and the tray goes inert
    // again, so fail loudly rather than silently skipping the wiring.
    let Some(tray) = app.tray_by_id("main") else {
        eprintln!(
            "[vibeaichat] no tray icon with id \"main\" — the tray will not respond to clicks"
        );
        return Ok(());
    };

    // macOS draws a template image from its alpha channel alone and ignores the
    // colours entirely. `icons/icon.png` is 96% opaque — a filled rounded square —
    // so as a template it rendered in the menu bar as a plain white square.
    // `tray-template.png` is the brand V on transparency, which is what the
    // menu bar (and every other icon beside it) actually wants. Windows and
    // Linux keep the full-colour icon from tauri.conf.json.
    #[cfg(target_os = "macos")]
    {
        let bytes: &[u8] = include_bytes!("../icons/tray-template.png");
        match tauri::image::Image::from_bytes(bytes) {
            // One call, not set_icon + set_icon_as_template, which renders twice.
            Ok(img) => tray.set_icon_with_as_template(Some(img), true)?,
            Err(e) => eprintln!("[vibeaichat] tray template icon failed to decode: {e}"),
        }
    }

    tray.set_menu(Some(menu))?;
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "show" => reveal(app),
        "quit" => app.exit(0),
        _ => {}
    });
    tray.on_tray_icon_event(|tray, event| {
        // Only the release of a left click; press-and-release would fire twice.
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let app = tray.app_handle();
            match app.get_webview_window("main") {
                Some(w) if w.is_visible().unwrap_or(false) => {
                    let _ = w.hide();
                }
                _ => reveal(app),
            }
        }
    });
    Ok(())
}

/// Show and focus the main window. `show()` alone leaves it behind whatever the
/// user is looking at, which reads as the click having done nothing.
fn reveal(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    /// macOS draws a template image from its alpha channel alone, so an icon
    /// that is mostly opaque renders as a solid block whatever its colours are.
    /// Shipping the app icon (96% opaque) as the tray template is what put a
    /// plain white square in the menu bar; this pins the replacement's shape.
    #[test]
    fn tray_template_is_a_glyph_not_a_filled_square() {
        let bytes: &[u8] = include_bytes!("../icons/tray-template.png");
        let img = tauri::image::Image::from_bytes(bytes).expect("tray template decodes as PNG");

        let total = (img.width() * img.height()) as usize;
        let opaque = img
            .rgba()
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|&&a| a == 255)
            .count();

        assert!(
            opaque * 2 < total,
            "tray template is {opaque}/{total} fully opaque — a template icon that \
             covers most of its canvas renders as a filled square in the menu bar"
        );
    }
}
