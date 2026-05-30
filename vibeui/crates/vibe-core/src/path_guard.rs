//! Path-traversal gate — DREAD #2 single source of truth.
//!
//! VibeCody accepts paths from multiple untrusted-or-low-trust
//! surfaces:
//!
//! * Tauri commands invoked by the WebView (`vibeui/src-tauri/...`)
//! * Daemon HTTP routes invoked by mobile / watch / plugin clients
//!   (`vibecli/vibecli-cli/src/serve.rs`)
//! * `vibe-indexer` HTTP routes (a standalone sidecar)
//! * MCP server `tools/call` dispatch (`mcp_server.rs::call_tool`)
//!
//! Each of those would, without a gate, let a caller resolve a path
//! into a credential directory (`~/.vibecli`, `~/.ssh`, `~/.aws`,
//! `~/.gnupg`, `~/.claude`) or a credential filename
//! (`profile_settings.db`, `id_rsa`, `credentials`, …) and have a
//! tool open / walk / write it.
//!
//! Prior to this module the helper existed as four near-identical
//! copies (Tauri commands, `vibecli-cli`, `vibe-indexer`, `mcp_server`).
//! Promoting it here makes drift impossible.
//!
//! ## Behavior
//!
//! * `canonicalize_lenient` walks the path back to its deepest
//!   existing ancestor, canonicalizes that, then re-joins the tail.
//!   That resolves `..` segments and symlinks the kernel knows about,
//!   without requiring the leaf to exist (necessary for create-file
//!   call sites).
//! * `DENIED_SEGMENTS` rejects any normalized component matching a
//!   well-known credential-directory name (case-insensitive).
//! * `DENIED_FILENAMES` rejects targets whose final component matches
//!   a well-known credential filename.
//!
//! Match is case-insensitive: macOS APFS and Windows NTFS are
//! case-insensitive by default, so `~/.AWS/credentials` and
//! `~/.aws/credentials` reach the same file.
//!
//! See `docs/security/threat-model.md` §8 row #2.

use std::path::{Path, PathBuf};

/// Directories that hold credentials or daemon state. Any normalized
/// component matching one of these (case-insensitively) is denied.
pub const DENIED_SEGMENTS: &[&str] = &[".vibecli", ".vibeui", ".claude", ".ssh", ".aws", ".gnupg"];

/// Filenames that hold credentials directly. Any path whose final
/// component matches one of these (case-insensitively) is denied,
/// even if the parent directory looks neutral.
pub const DENIED_FILENAMES: &[&str] = &[
    "daemon.token",
    "profile_settings.db",
    "workspace.db",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials",
    "config.json",
];

/// Canonicalize a path even if its leaf doesn't exist.
///
/// `std::fs::canonicalize` fails when the target file is missing —
/// which is the common case for write-side call sites (the gate runs
/// *before* the write creates the file). This helper walks back to
/// the deepest existing ancestor, canonicalizes that, then re-joins
/// the missing tail so the resolved-symlinks / collapsed-`..` form
/// of the existing prefix carries through.
pub fn canonicalize_lenient(path: &Path) -> Result<PathBuf, std::io::Error> {
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

/// Reject paths that traverse a credential directory or target a
/// credential filename.
///
/// Returns the canonicalized form on success so callers can do
/// subsequent fs operations against the symlink-resolved /
/// `..`-collapsed path. On failure the error string includes both
/// the original input and the matched deny-list entry so the audit
/// log shows exactly which guard tripped.
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
    fn accepts_neutral_cwd() {
        let cwd = std::env::current_dir().expect("cwd");
        let result = reject_sensitive_path(cwd.to_str().unwrap());
        assert!(
            result.is_ok(),
            "neutral cwd should be allowed: {:?}",
            result
        );
    }

    #[test]
    fn rejects_dot_vibecli_segment() {
        let err = reject_sensitive_path("/tmp/.vibecli").unwrap_err();
        assert!(
            err.contains(".vibecli"),
            "expected mention of .vibecli: {err}"
        );
    }

    #[test]
    fn rejects_dot_vibeui_segment() {
        let err = reject_sensitive_path("/tmp/.vibeui/api_keys.json").unwrap_err();
        assert!(err.contains(".vibeui"), "got: {err}");
    }

    #[test]
    fn rejects_dot_claude_segment() {
        let err = reject_sensitive_path("/tmp/.claude/projects").unwrap_err();
        assert!(err.contains(".claude"), "got: {err}");
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
    fn rejects_dot_ssh_segment() {
        let err = reject_sensitive_path("/tmp/.ssh/id_rsa").unwrap_err();
        assert!(err.contains(".ssh") || err.contains("id_rsa"), "got: {err}");
    }

    #[test]
    fn rejects_dot_gnupg_segment() {
        let err = reject_sensitive_path("/tmp/.gnupg").unwrap_err();
        assert!(err.contains(".gnupg"), "got: {err}");
    }

    #[test]
    fn rejects_credential_filenames() {
        let err = reject_sensitive_path("/tmp/profile_settings.db").unwrap_err();
        assert!(err.contains("profile_settings.db"), "got: {err}");
    }

    #[test]
    fn rejects_id_rsa_in_neutral_dir() {
        let err = reject_sensitive_path("/tmp/id_rsa").unwrap_err();
        assert!(err.contains("id_rsa"), "got: {err}");
    }

    #[test]
    fn rejects_workspace_db_in_neutral_dir() {
        let err = reject_sensitive_path("/tmp/workspace.db").unwrap_err();
        assert!(err.contains("workspace.db"), "got: {err}");
    }

    #[test]
    fn case_insensitive_segment_match() {
        let err = reject_sensitive_path("/tmp/.SSH/id_rsa").unwrap_err();
        assert!(err.contains(".ssh") || err.contains("id_rsa"), "got: {err}");
    }

    #[test]
    fn case_insensitive_filename_match() {
        let err = reject_sensitive_path("/tmp/PROFILE_SETTINGS.DB").unwrap_err();
        assert!(err.contains("profile_settings.db"), "got: {err}");
    }

    #[test]
    fn lookalike_segment_accepted() {
        // `.awsness` is not `.aws` — the deny-list matches whole
        // components, not prefixes.
        let tmp = std::env::temp_dir().join(".awsness");
        let result = reject_sensitive_path(tmp.to_str().unwrap());
        assert!(
            result.is_ok(),
            "lookalike `.awsness` should be allowed: {result:?}"
        );
    }

    #[test]
    fn canonicalize_lenient_handles_nonexistent_leaf() {
        // Write-side call sites pass a path that doesn't exist yet.
        // The helper must canonicalize the existing ancestor and
        // re-append the tail rather than erroring.
        let tmp = std::env::temp_dir().join("vibecore_path_guard_test_nonexistent");
        let result = canonicalize_lenient(&tmp).expect("lenient canonicalize");
        // The leaf doesn't exist, so canonical contains the tail
        // verbatim under whatever the canonical temp_dir resolves to.
        assert!(
            result.ends_with("vibecore_path_guard_test_nonexistent"),
            "got: {}",
            result.display()
        );
    }
}
