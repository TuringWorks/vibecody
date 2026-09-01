//! Console-free child process construction for crates that are compiled into
//! **both** GUI apps and console apps.
//!
//! On Windows, when a process that has no console attached spawns a
//! console-subsystem child (`git`, `sh`, `cmd`, a language server, ...), the OS
//! allocates a fresh console *window* for that child. The command finishes in
//! milliseconds and the window vanishes, which reads to the user as the desktop
//! flashing black boxes at them. `CREATE_NO_WINDOW` suppresses that allocation.
//!
//! # Why this is conditional, unlike `vibecoder/src-tauri`'s `no_window`
//!
//! That module can set the flag unconditionally, because that binary is only
//! ever a GUI (`windows_subsystem = "windows"`). The crates that use *this* one
//! — `vibe-core`, `vibe-ai`, `vibe-lsp` — are linked into the VibeCoder GUI
//! **and** into `vibecli` and `vibe-indexer`, which are console applications.
//! Setting `CREATE_NO_WINDOW` unconditionally there would cut a child off from
//! a console its parent legitimately owns.
//!
//! So the flag is set only when the current process has no console of its own,
//! which is exactly the case where Windows would otherwise allocate a window:
//!
//! | Host process                    | `GetConsoleWindow()` | flag set |
//! |---------------------------------|----------------------|----------|
//! | VibeCoder GUI (release bundle)  | null                 | yes      |
//! | `vibecli` in a terminal         | non-null             | no       |
//! | `vibecli` under a GUI / service | null                 | yes      |
//! | `cargo test`, `cargo run`       | non-null             | no       |
//!
//! Stdio redirection is unaffected either way: `.output()` and piped stdio
//! capture identically, because pipes do not depend on the child owning a
//! console.
//!
//! On non-Windows targets every function here is exactly `Command::new`.

use std::ffi::OsStr;

/// <https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags>
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Whether spawning a console-subsystem child would make Windows allocate a
/// console window — i.e. whether this process has no console of its own.
///
/// Not cached: `GetConsoleWindow` reads process-local state and costs orders of
/// magnitude less than the `CreateProcess` that follows it, and a cache would
/// go stale for anything that calls `AllocConsole` / `FreeConsole` later.
///
/// <https://learn.microsoft.com/en-us/windows/console/getconsolewindow>
#[cfg(windows)]
fn spawning_would_allocate_a_console() -> bool {
    // Declared here rather than pulled from `windows-sys`, so this crate stays
    // dependency-free for its std-only consumers. `kernel32` is named
    // explicitly rather than relying on std having already linked it, so this
    // cannot become an undefined symbol if that ever changes.
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> *mut core::ffi::c_void;
    }
    // Safe: takes no arguments, touches no memory we own, and the returned
    // window handle is only compared against null.
    unsafe { GetConsoleWindow() }.is_null()
}

/// `std::process::Command::new`, minus the console window on Windows.
///
/// Returns an owned `Command`, so it is drop-in at both inline call chains and
/// `let mut cmd = ...` bindings.
pub fn std_command<S: AsRef<OsStr>>(program: S) -> std::process::Command {
    // Off Windows nothing below mutates `cmd`, and `make lint` runs clippy with
    // `-D warnings` on macOS and Linux too, so `unused_mut` would be an error
    // on exactly the platforms this function is a no-op for.
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        if spawning_would_allocate_a_console() {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
    }
    cmd
}

/// `tokio::process::Command::new`, minus the console window on Windows.
#[cfg(feature = "tokio")]
pub fn tokio_command<S: AsRef<OsStr>>(program: S) -> tokio::process::Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    {
        if spawning_would_allocate_a_console() {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructors must still produce a working, capturable child. A
    /// wrong creation flag shows up here as a spawn failure or empty stdout.
    #[test]
    fn std_command_still_captures_stdout() {
        let (prog, args): (&str, &[&str]) = if cfg!(windows) {
            ("cmd", &["/C", "echo vibe"])
        } else {
            ("sh", &["-c", "echo vibe"])
        };
        let out = std_command(prog).args(args).output().expect("spawn");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "vibe");
    }

    /// `cargo test` owns a console, so on Windows the flag must *not* be set
    /// here. This is the branch that keeps `vibecli` usable in a terminal.
    #[cfg(windows)]
    #[test]
    fn a_process_with_a_console_is_left_alone() {
        assert!(
            !spawning_would_allocate_a_console(),
            "cargo test has a console, so no window would be allocated"
        );
    }
}
