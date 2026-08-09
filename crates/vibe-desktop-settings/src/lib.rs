//! Settings commands shared by the VibeCody desktop shells.
//!
//! `#[tauri::command]` functions do not have to live in the app crate — they
//! only have to be nameable from `generate_handler!`. Keeping them here means
//! VibeDesk and VibeAIChat drive the *same* ProfileStore code, so a fix to key
//! storage cannot land in one shell and miss the other.
//!
//! The UI side of these screens lives in `packages/vibe-ui-shared`. A shell
//! that renders those screens without registering these commands compiles and
//! renders fine, and every control silently does nothing — so wire both.

pub mod settings;

// Re-exported flat so a shell can write `vibe_desktop_settings::setting_get`
// in `generate_handler!` without naming the module.
pub use settings::*;
