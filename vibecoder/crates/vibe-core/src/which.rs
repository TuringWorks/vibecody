//! Resolving a program name against `PATH`.
//!
//! This exists because the obvious shell one-liner is not portable. `sh -c
//! "command -v npm"` needs a POSIX shell, which Windows does not have unless
//! Git for Windows happens to be installed — so the probe that asks "is this
//! tool available?" was itself unavailable on the platform most likely to be
//! missing the tool.
//!
//! On Windows a bare name is not enough either: `npm`, `npx`, `yarn` and most
//! of the Node ecosystem install as `npm.cmd`, not `npm.exe`. Looking only for
//! the literal name (or only for `.exe`) reports "not installed" for tools that
//! are installed and on `PATH`. `PATHEXT` is the list Windows itself uses, so
//! that is the list we use.

use std::path::{Path, PathBuf};

/// The extensions Windows appends when resolving a bare program name, used
/// when `PATHEXT` is unset or unreadable.
#[cfg(windows)]
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// Extensions to try after the literal name, in `PATHEXT` order.
#[cfg(windows)]
fn path_extensions() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| DEFAULT_PATHEXT.to_string())
        .split(';')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect()
}

/// Candidate file names for `program` inside a single `PATH` directory.
#[cfg(not(windows))]
fn candidates(program: &str) -> Vec<PathBuf> {
    vec![PathBuf::from(program)]
}

/// Candidate file names for `program` inside a single `PATH` directory: the
/// literal name first, then each `PATHEXT` extension.
#[cfg(windows)]
fn candidates(program: &str) -> Vec<PathBuf> {
    let literal = PathBuf::from(program);
    // A name that already carries an extension is used as written; appending
    // `.exe` to `foo.cmd` would look for `foo.cmd.exe`.
    if literal.extension().is_some() {
        return vec![literal];
    }
    std::iter::once(literal.clone())
        .chain(path_extensions().into_iter().map(|ext| {
            let mut with_ext = literal.clone();
            with_ext.set_extension(ext);
            with_ext
        }))
        .collect()
}

/// The absolute path `program` resolves to on `PATH`, or `None` if it is not
/// there. An absolute or relative path is checked directly rather than
/// searched, matching how the OS treats it.
pub fn on_path(program: &str) -> Option<PathBuf> {
    if program.is_empty() {
        return None;
    }

    let as_path = Path::new(program);
    if as_path.is_absolute() || as_path.components().count() > 1 {
        return as_path.is_file().then(|| as_path.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    let names = candidates(program);
    std::env::split_paths(&path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// Whether `program` can be run without an absolute path.
pub fn is_on_path(program: &str) -> bool {
    on_path(program).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every platform this ships on has the interpreter that runs the tests.
    #[test]
    fn finds_a_program_that_is_certainly_installed() {
        let known = if cfg!(windows) { "cmd" } else { "sh" };
        let found = on_path(known).unwrap_or_else(|| panic!("{known} should be on PATH"));
        assert!(found.is_file(), "{} should be a file", found.display());
    }

    #[test]
    fn absent_program_is_none() {
        assert!(on_path("vibecody-definitely-not-a-real-program").is_none());
    }

    #[test]
    fn empty_name_is_none() {
        assert!(on_path("").is_none());
    }

    /// A path, rather than a bare name, must not be searched for in every
    /// `PATH` directory — that would resolve `./evil` to something unrelated.
    #[test]
    fn a_path_is_checked_directly_not_searched() {
        let missing = std::env::temp_dir().join("vibecody-not-here").join("tool");
        assert!(on_path(&missing.to_string_lossy()).is_none());
    }

    /// `PATHEXT` is the reason `npm` resolves on Windows at all: the file is
    /// `npm.cmd`. Nothing to assert off Windows, where the name is literal.
    #[cfg(windows)]
    #[test]
    fn windows_tries_pathext_after_the_literal_name() {
        let rendered: Vec<String> = candidates("npm")
            .iter()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .collect();
        assert_eq!(rendered.first().map(String::as_str), Some("npm"));
        assert!(
            rendered.iter().any(|n| n == "npm.cmd"),
            "PATHEXT should contribute npm.cmd, got {rendered:?}"
        );
    }

    /// An explicit extension is taken as written.
    #[cfg(windows)]
    #[test]
    fn windows_does_not_append_to_an_explicit_extension() {
        assert_eq!(candidates("npm.cmd"), vec![PathBuf::from("npm.cmd")]);
    }
}
