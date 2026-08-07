//! Locating language-server binaries, and building the `PATH` we hand them.
//!
//! A GUI app launched from Finder / Dock inherits launchd's `PATH`
//! (`/usr/bin:/bin:/usr/sbin:/sbin`) — not the shell's. Every language server
//! anyone actually installs lives somewhere else: `rust-analyzer` in
//! `~/.cargo/bin`, `pyright-langserver` in an npm prefix, `clangd` in
//! `/opt/homebrew/bin`. So a `Command::new("rust-analyzer")` that works in
//! `cargo run` fails in the bundled `.app` with `No such file or directory`,
//! and every completion request returns an error.
//!
//! Two things fix that, both here:
//!   * [`resolve_server`] finds the binary across the well-known install dirs
//!     as well as `PATH`, so we can spawn it by absolute path.
//!   * [`augmented_path`] hands the server a `PATH` containing those same dirs,
//!     because servers shell out too (`rust-analyzer` → `cargo`,
//!     `typescript-language-server` → `node`).
//!
//! The search itself is a pure function over an explicit set of roots
//! ([`ServerSearchPaths`]) so it can be tested without touching the
//! environment.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Where to look for a language-server binary.
#[derive(Debug, Clone, Default)]
pub struct ServerSearchPaths {
    /// `PATH`, split into entries, in order.
    pub path_entries: Vec<PathBuf>,
    /// Well-known install prefixes that a GUI-inherited `PATH` usually lacks.
    pub extra_prefixes: Vec<PathBuf>,
}

impl ServerSearchPaths {
    /// Read the current environment.
    pub fn from_env() -> Self {
        let path_entries = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        Self {
            path_entries,
            extra_prefixes: default_prefixes(home_dir().as_deref()),
        }
    }

    /// Every candidate directory, `PATH` first, in resolution order.
    pub fn directories(&self) -> impl Iterator<Item = &PathBuf> {
        self.path_entries.iter().chain(self.extra_prefixes.iter())
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let var = "USERPROFILE";
    #[cfg(not(windows))]
    let var = "HOME";
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

/// Well-known language-server install directories, per platform.
///
/// Pure in `home` so the list is testable.
pub fn default_prefixes(home: Option<&Path>) -> Vec<PathBuf> {
    #[cfg(windows)]
    let system: Vec<PathBuf> = Vec::new();
    #[cfg(not(windows))]
    let system: Vec<PathBuf> = [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/opt/local/bin",
        "/usr/bin",
        "/bin",
        // Swift: sourcekit-lsp ships inside the toolchain, never on a GUI PATH.
        "/Applications/Xcode.app/Contents/Developer/usr/bin",
        "/Library/Developer/CommandLineTools/usr/bin",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();

    // Per-user tool dirs, in rough order of how often they hold a server.
    #[cfg(windows)]
    let relative = [
        ".cargo/bin",
        "scoop/shims",
        "go/bin",
        "AppData/Roaming/npm",
        ".dotnet/tools",
    ];
    #[cfg(not(windows))]
    let relative = [
        ".cargo/bin",
        ".local/bin",
        "go/bin",
        ".bun/bin",
        ".deno/bin",
        ".volta/bin",
        ".npm-global/bin",
        ".yarn/bin",
        ".rbenv/shims",
        ".pyenv/shims",
        ".dotnet/tools",
        ".ghcup/bin",
        ".opam/default/bin",
        ".pub-cache/bin",
    ];

    let user = home.into_iter().flat_map(|h| {
        relative
            .iter()
            .map(move |r| r.split('/').fold(h.to_path_buf(), |acc, seg| acc.join(seg)))
    });

    user.chain(system).collect()
}

/// Candidate file names for `cmd` on this platform.
fn candidate_names(cmd: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        // A bare name may be a .exe, or a shim (.cmd/.bat) — npm installs those.
        let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        let has_ext = Path::new(cmd).extension().is_some();
        if has_ext {
            return vec![cmd.to_string()];
        }
        std::iter::once(cmd.to_string())
            .chain(
                exts.split(';')
                    .filter(|e| !e.is_empty())
                    .map(|e| format!("{cmd}{}", e.to_lowercase())),
            )
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![cmd.to_string()]
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Resolve a language-server command to an absolute path.
///
/// An absolute or explicitly-relative `cmd` (`/usr/bin/clangd`, `./zls`) is
/// checked as-is and never searched. Returns `None` when nothing executable
/// matches — the caller reports "not installed" rather than spawning and
/// failing.
pub fn resolve_server(cmd: &str, paths: &ServerSearchPaths) -> Option<PathBuf> {
    let as_path = Path::new(cmd);
    if as_path.is_absolute() || cmd.contains('/') || cmd.contains('\\') {
        return is_executable_file(as_path).then(|| as_path.to_path_buf());
    }

    let names = candidate_names(cmd);
    paths
        .directories()
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| is_executable_file(candidate))
}

/// Is this server installed? Cheaper than spawning it to find out.
pub fn server_available(cmd: &str, paths: &ServerSearchPaths) -> bool {
    resolve_server(cmd, paths).is_some()
}

/// The `PATH` to give a spawned language server: the inherited one, plus every
/// well-known prefix that isn't already in it, duplicates removed.
pub fn augmented_path(paths: &ServerSearchPaths) -> OsString {
    let mut seen = std::collections::HashSet::new();
    let ordered: Vec<&PathBuf> = paths
        .directories()
        .filter(|dir| !dir.as_os_str().is_empty() && seen.insert((*dir).clone()))
        .collect();
    std::env::join_paths(ordered).unwrap_or_else(|_| {
        std::env::var_os("PATH").unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch_exe(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\n").expect("write fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fixture");
        }
        path
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vibe-lsp-discovery-{}-{}-{tag}",
            std::process::id(),
            tag.len()
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    #[test]
    fn resolves_from_path_entries() {
        let dir = tmpdir("path-entry");
        touch_exe(&dir, "fake-analyzer");
        let paths = ServerSearchPaths {
            path_entries: vec![dir.clone()],
            extra_prefixes: vec![],
        };
        assert_eq!(
            resolve_server("fake-analyzer", &paths),
            Some(dir.join("fake-analyzer"))
        );
    }

    #[test]
    fn resolves_from_extra_prefix_when_path_lacks_it() {
        // The GUI-launch case: PATH is useless, the prefix list saves us.
        let dir = tmpdir("extra-prefix");
        touch_exe(&dir, "fake-gopls");
        let paths = ServerSearchPaths {
            path_entries: vec![PathBuf::from("/nonexistent-bin")],
            extra_prefixes: vec![dir.clone()],
        };
        assert_eq!(
            resolve_server("fake-gopls", &paths),
            Some(dir.join("fake-gopls"))
        );
    }

    #[test]
    fn path_entries_win_over_extra_prefixes() {
        let first = tmpdir("prefer-a");
        let second = tmpdir("prefer-bb");
        touch_exe(&first, "dup-server");
        touch_exe(&second, "dup-server");
        let paths = ServerSearchPaths {
            path_entries: vec![first.clone()],
            extra_prefixes: vec![second],
        };
        assert_eq!(
            resolve_server("dup-server", &paths),
            Some(first.join("dup-server"))
        );
    }

    #[test]
    fn missing_server_is_none() {
        let paths = ServerSearchPaths {
            path_entries: vec![PathBuf::from("/nonexistent-bin")],
            extra_prefixes: vec![],
        };
        assert!(resolve_server("definitely-not-installed-xyz", &paths).is_none());
        assert!(!server_available("definitely-not-installed-xyz", &paths));
    }

    #[test]
    fn non_executable_file_is_not_a_server() {
        let dir = tmpdir("not-exec");
        let path = dir.join("readme-only");
        std::fs::write(&path, b"not a program").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
                .expect("chmod");
        }
        let paths = ServerSearchPaths {
            path_entries: vec![dir],
            extra_prefixes: vec![],
        };
        #[cfg(unix)]
        assert!(resolve_server("readme-only", &paths).is_none());
    }

    #[test]
    fn directory_is_not_a_server() {
        let dir = tmpdir("dir-not-exe");
        std::fs::create_dir_all(dir.join("subdir")).expect("mkdir");
        let paths = ServerSearchPaths {
            path_entries: vec![dir],
            extra_prefixes: vec![],
        };
        assert!(resolve_server("subdir", &paths).is_none());
    }

    #[test]
    fn absolute_command_is_used_verbatim() {
        let dir = tmpdir("absolute");
        let exe = touch_exe(&dir, "abs-server");
        let empty = ServerSearchPaths::default();
        assert_eq!(
            resolve_server(exe.to_str().expect("utf8"), &empty),
            Some(exe)
        );
    }

    #[test]
    fn absolute_command_that_does_not_exist_is_none() {
        let empty = ServerSearchPaths::default();
        assert!(resolve_server("/nonexistent/abs-server", &empty).is_none());
    }

    #[test]
    fn default_prefixes_include_cargo_bin_for_rust_analyzer() {
        let home = PathBuf::from("/home/dev");
        let prefixes = default_prefixes(Some(&home));
        assert!(prefixes.contains(&home.join(".cargo").join("bin")));
    }

    #[test]
    fn default_prefixes_without_home_still_has_system_dirs() {
        let prefixes = default_prefixes(None);
        assert!(!prefixes.is_empty());
        #[cfg(not(windows))]
        assert!(prefixes.contains(&PathBuf::from("/usr/bin")));
    }

    #[test]
    fn augmented_path_dedupes_and_keeps_order() {
        let paths = ServerSearchPaths {
            path_entries: vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")],
            extra_prefixes: vec![PathBuf::from("/usr/bin"), PathBuf::from("/opt/tools/bin")],
        };
        let joined = augmented_path(&paths);
        let entries: Vec<PathBuf> = std::env::split_paths(&joined).collect();
        assert_eq!(
            entries,
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/bin"),
                PathBuf::from("/opt/tools/bin")
            ]
        );
    }

    #[test]
    fn augmented_path_includes_extra_prefixes() {
        let paths = ServerSearchPaths {
            path_entries: vec![PathBuf::from("/usr/bin")],
            extra_prefixes: vec![PathBuf::from("/home/dev/.cargo/bin")],
        };
        let joined = augmented_path(&paths);
        let entries: Vec<PathBuf> = std::env::split_paths(&joined).collect();
        assert!(entries.contains(&PathBuf::from("/home/dev/.cargo/bin")));
    }
}
