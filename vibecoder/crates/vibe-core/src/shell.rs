//! Running a POSIX shell command, including on Windows.
//!
//! Every command string this codebase hands to a shell is POSIX: the agent's
//! `bash` tool, the build/run commands detected for a project (`npm run build
//! && npm test`), the deploy commands a user types, the lint invocations in a
//! harness profile. Those strings use pipes, `&&`, `>/dev/null`, `$(…)` and
//! POSIX quoting, so translating them to `cmd /C` would not run them, it would
//! mis-run them — silently, with the wrong result rather than an error. The
//! shell stays `sh` on every platform.
//!
//! What was broken on Windows is *finding* `sh`. Git for Windows ships a
//! complete `sh.exe`, but its installer adds only `<Git>\cmd` to `PATH` by
//! default — the directory with `git.exe` in it and nothing else. So the usual
//! Windows developer has git working and `sh` missing, and every call site
//! failed with a bare "program not found" that named neither the cause nor the
//! fix. [`posix_shell`] therefore looks past `PATH` into the places Git
//! actually installs, and [`explain`] turns the remaining not-found case into a
//! sentence that says what to install.
//!
//! Resolution is cached: these constructors sit in front of every build, test
//! and agent command, and the answer does not change under a running process.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where Git for Windows puts `sh.exe`, in the order to prefer them.
///
/// `usr\bin` is the full MSYS2 environment (the one `git-bash.exe` opens);
/// `bin` is the smaller wrapper set. Both are real shells, and a machine with
/// Git installed has at least one.
#[cfg(windows)]
const GIT_FOR_WINDOWS_SHELLS: &[&str] = &[
    r"C:\Program Files\Git\usr\bin\sh.exe",
    r"C:\Program Files\Git\bin\sh.exe",
    r"C:\Program Files (x86)\Git\usr\bin\sh.exe",
    r"C:\Program Files (x86)\Git\bin\sh.exe",
];

/// `sh.exe` relative to the directory holding `git.exe`, for a Git installed
/// somewhere other than Program Files. `<Git>\cmd\git.exe` and
/// `<Git>\bin\git.exe` are both shipped layouts, so walk up from whichever we
/// found and look in the sibling shell directories.
#[cfg(windows)]
fn shell_beside_git() -> Option<PathBuf> {
    let git = crate::which::on_path("git")?;
    let root = git.parent()?.parent()?;
    ["usr/bin/sh.exe", "bin/sh.exe"]
        .iter()
        .map(|rel| root.join(rel))
        .find(|candidate| candidate.is_file())
}

/// The shell to use on Windows: Git for Windows first, `PATH` only after.
///
/// Deliberately not `PATH`-first, unlike Unix. On Windows a bare `sh.exe` on
/// `PATH` is very often a package-manager shim with no POSIX environment
/// behind it -- Chocolatey installs one that is a zsh wrapper, and `sleep`,
/// `grep` and `sed` are all unreachable from it. A shell like that runs
/// `echo` and then fails on the first real command, one command at a time,
/// with errors that look like the project's fault. Git for Windows ships the
/// complete MSYS2 environment, and the product already requires git.
///
/// A `PATH` shell is still used when there is no Git install, because a
/// deliberate MSYS2 or Cygwin setup is a better answer than nothing.
#[cfg(windows)]
fn resolve_shell() -> Option<PathBuf> {
    shell_beside_git()
        .or_else(|| {
            GIT_FOR_WINDOWS_SHELLS
                .iter()
                .map(PathBuf::from)
                .find(|candidate| candidate.is_file())
        })
        .or_else(|| crate::which::on_path("sh"))
}

/// The shell to use off Windows: whatever `PATH` says, and `/bin/sh` after
/// that. A Unix box with no `sh` on `PATH` is broken, but POSIX still
/// guarantees the shell's location.
#[cfg(not(windows))]
fn resolve_shell() -> Option<PathBuf> {
    crate::which::on_path("sh").or_else(|| {
        let fallback = PathBuf::from("/bin/sh");
        fallback.is_file().then_some(fallback)
    })
}

/// The POSIX shell to run commands with, if this machine has one.
///
/// `None` is a real answer, not a failure to look: a Windows box without Git
/// for Windows genuinely has no `sh`, and callers should say so rather than
/// pretend a spawn could have worked.
pub fn posix_shell() -> Option<&'static Path> {
    static RESOLVED: OnceLock<Option<PathBuf>> = OnceLock::new();
    RESOLVED.get_or_init(resolve_shell).as_deref()
}

/// The program to spawn: the resolved shell, or the bare name so the OS
/// produces the same not-found error it always did rather than us inventing a
/// different failure mode.
fn shell_program() -> PathBuf {
    posix_shell().map_or_else(|| PathBuf::from("sh"), Path::to_path_buf)
}

/// `sh -c <command>`, console-window-free, ready for `.current_dir()` and the
/// rest of the builder.
pub fn sh(command: &str) -> std::process::Command {
    let mut cmd = vibe_no_window::std_command(shell_program());
    cmd.arg("-c").arg(command);
    cmd
}

/// The `tokio::process` form of [`sh`].
pub fn sh_async(command: &str) -> tokio::process::Command {
    let mut cmd = vibe_no_window::tokio_command(shell_program());
    cmd.arg("-c").arg(command);
    cmd
}

/// What to tell the user when spawning a shell command failed.
///
/// A missing shell and a missing *tool* produce the same `NotFound` from the
/// OS, but only one of them is fixed by installing Git. Checking which we are
/// looking at is the difference between advice and noise.
pub fn explain(error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound && posix_shell().is_none() {
        return missing_shell_message();
    }
    error.to_string()
}

/// The message for a machine with no POSIX shell at all.
pub fn missing_shell_message() -> String {
    if cfg!(windows) {
        "no POSIX shell found: project and agent commands are POSIX and run \
         with `sh`, which on Windows comes from Git for Windows. Install it \
         from https://git-scm.com/download/win, or put a directory containing \
         `sh.exe` on PATH."
            .to_string()
    } else {
        "no POSIX shell found: `sh` is not on PATH and /bin/sh does not exist.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whatever we resolve has to actually run a command, and run it as a
    /// shell. This is the whole point of the module, and `&&` is the assertion
    /// that would have caught `cmd /C` being substituted for it.
    #[test]
    fn the_resolved_shell_runs_a_posix_command() {
        let Some(found) = posix_shell() else {
            // A machine with no shell is a real configuration; the constructor
            // contract still holds, there is just nothing to execute.
            assert!(
                !missing_shell_message().is_empty(),
                "a missing shell must still explain itself"
            );
            return;
        };
        assert!(found.is_file(), "{} should be a file", found.display());

        let out = sh("echo vibe && echo cody").output().expect("spawn sh");
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
        assert_eq!(lines, vec!["vibe", "cody"], "`&&` must be shell syntax");
    }

    /// POSIX redirection is the other half of what `cmd /C` cannot do.
    #[test]
    fn the_resolved_shell_understands_posix_redirection() {
        if posix_shell().is_none() {
            return;
        }
        let out = sh("echo loud >/dev/null 2>&1; echo quiet")
            .output()
            .expect("spawn sh");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "quiet");
    }

    /// `explain` must not blame the shell for a tool that is missing while the
    /// shell itself is present.
    #[test]
    fn a_not_found_with_a_shell_present_is_not_a_missing_shell() {
        if posix_shell().is_none() {
            return;
        }
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "cargo-nonexistent");
        assert_eq!(explain(&err), "cargo-nonexistent");
    }

    /// Resolution is cached, so it must be stable across calls.
    #[test]
    fn resolution_is_stable() {
        assert_eq!(posix_shell(), posix_shell());
    }
}
