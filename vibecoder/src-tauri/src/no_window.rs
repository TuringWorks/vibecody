//! Console-free child process construction.
//!
//! `main.rs` builds this binary with `windows_subsystem = "windows"`, so in
//! release the app has no console attached. On Windows, spawning a
//! console-subsystem child (`git`, `sh`, `sqlite3`, `psql`, ...) from a process
//! with no console makes the OS allocate a fresh console *window* for the
//! child. The command finishes in milliseconds and the window vanishes, which
//! reads to the user as the desktop flashing black boxes at them.
//!
//! The implementation lives in the `vibe-no-window` crate, because the shared
//! crates that run inside this same process (`vibe-core`, `vibe-ai`,
//! `vibe-lsp`) spawn children too and are *also* linked into `vibecli` and
//! `vibe-indexer`, which are console applications. One implementation, one
//! policy: suppress the window only when this process has no console of its
//! own. In a release bundle that is always true, so the GUI behaves exactly as
//! it would under an unconditional flag; under `npm run tauri:dev` the dev
//! console is inherited, so child output stays visible in the terminal.
//!
//! This module stays as the crate-local name because 181 call sites spell it
//! `crate::no_window::…`, and because it is the obvious place to look from
//! inside this crate.

pub use vibe_no_window::{std_command, tokio_command};
