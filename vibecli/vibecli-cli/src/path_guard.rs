//! Path-traversal gate for the daemon's HTTP boundary — DREAD #2.
//!
//! The daemon accepts `workspace` paths from external clients
//! (mobile / watch / plugins, anything holding the daemon token).
//! Without a gate, a malicious or compromised client could create
//! an agent task rooted at `~/.aws`, `~/.ssh`, or `~/.vibecli` and
//! have the agent loop's read / write / shell tools operate inside
//! a credential directory.
//!
//! This module mirrors the helper of the same name in
//! `vibeui/src-tauri/src/commands.rs` — kept in two crates because
//! the Tauri wrapper and the CLI crate are independent and `vibe_core`
//! is the shared substrate, but that promotion can happen later
//! without changing the gate's call shape.
//!
//! ## Behavior
//!
//! * `canonicalize_lenient` resolves the path through any existing
//!   ancestor so symlinks / `..` are normalized even when the target
//!   itself doesn't exist yet (the daemon may be writing a new file).
//! * `DENIED_SEGMENTS` rejects any directory whose canonical form
//!   includes `.vibecli` / `.vibeui` / `.claude` / `.ssh` / `.aws` /
//!   `.gnupg` as a path component (case-insensitive).
//! * `DENIED_FILENAMES` rejects targets whose final component is a
//!   well-known credential filename (`daemon.token`,
//!   `profile_settings.db`, `id_rsa`, etc.).
//!
//! See `docs/security/threat-model.md` §8 row #2.

use std::path::{Path, PathBuf};

const DENIED_SEGMENTS: &[&str] = &[
    ".vibecli", ".vibeui", ".claude",
    ".ssh", ".aws", ".gnupg",
];

const DENIED_FILENAMES: &[&str] = &[
    "daemon.token", "profile_settings.db", "workspace.db",
    "id_rsa", "id_dsa", "id_ecdsa", "id_ed25519",
    "credentials", "config.json",
];

fn canonicalize_lenient(path: &Path) -> Result<PathBuf, std::io::Error> {
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let mut existing = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        let Some(file_name) = existing.file_name().map(|n| n.to_os_string()) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "path has no existing ancestor",
            ));
        };
        tail.push(file_name);
        if !existing.pop() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "path has no existing ancestor",
            ));
        }
    }
    let mut canonical = existing.canonicalize()?;
    for segment in tail.iter().rev() {
        canonical.push(segment);
    }
    Ok(canonical)
}

/// Reject paths that traverse known-sensitive directories or target
/// known-sensitive filenames. Returns the canonicalized path on
/// success so callers can use the validated form (resolved symlinks,
/// `..` normalized) for subsequent fs operations.
pub fn reject_sensitive_path(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let canonical = canonicalize_lenient(&p).unwrap_or(p);

    for component in canonical.components() {
        if let std::path::Component::Normal(seg) = component {
            let s = seg.to_string_lossy();
            for denied in DENIED_SEGMENTS {
                if s.eq_ignore_ascii_case(denied) {
                    return Err(format!(
                        "Access denied: '{path}' traverses sensitive directory '{denied}'"
                    ));
                }
            }
        }
    }
    if let Some(name) = canonical.file_name().and_then(|n| n.to_str()) {
        for denied in DENIED_FILENAMES {
            if name.eq_ignore_ascii_case(denied) {
                return Err(format!(
                    "Access denied: '{path}' targets sensitive file '{denied}'"
                ));
            }
        }
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_neutral_workspace() {
        // Use a path that exists on the test machine — the workspace root —
        // so canonicalize succeeds and we exercise the deny-list, not the
        // lenient fallback.
        let cwd = std::env::current_dir().expect("cwd");
        let result = reject_sensitive_path(cwd.to_str().unwrap());
        assert!(result.is_ok(), "neutral cwd should be allowed: {:?}", result);
    }

    #[test]
    fn rejects_dot_vibecli() {
        let err = reject_sensitive_path("/tmp/.vibecli").unwrap_err();
        assert!(err.contains(".vibecli"), "expected mention of .vibecli: {err}");
    }

    #[test]
    fn rejects_dot_aws_nested() {
        let err = reject_sensitive_path("/Users/example/.aws/credentials").unwrap_err();
        // Either segment or filename trip — both are legitimate failures.
        assert!(
            err.contains(".aws") || err.contains("credentials"),
            "expected sensitive marker in: {err}"
        );
    }

    #[test]
    fn rejects_credential_filenames() {
        let err = reject_sensitive_path("/tmp/profile_settings.db").unwrap_err();
        assert!(err.contains("profile_settings.db"), "got: {err}");
    }

    #[test]
    fn case_insensitive_segment_match() {
        let err = reject_sensitive_path("/tmp/.SSH/id_rsa").unwrap_err();
        assert!(err.contains(".ssh") || err.contains("id_rsa"), "got: {err}");
    }
}
