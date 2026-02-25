//! vibeapp — Tauri backend for the floating VibeCLI AI window.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::set_always_on_top,
            commands::start_drag,
            commands::hide_window,
            commands::show_window,
            commands::check_daemon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vibeapp");
}
