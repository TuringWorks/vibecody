//! What an agent may touch *outside* its workspace.
//!
//! The agent is normally jailed to `workspace_root`: `ToolExecutor::resolve_safe`
//! canonicalizes every path and rejects anything that escapes. Sandbox mode
//! relaxes that jail, which is the only place in the product where an LLM gets
//! reach beyond the project it was pointed at — so the relaxation is explicit,
//! per-axis, and denies by default.
//!
//! Three rules hold no matter how the policy is configured:
//!
//! 1. **Inside the workspace is unchanged.** The policy is consulted only for
//!    paths that escape the jail, so enabling sandbox mode cannot restrict what
//!    already worked.
//! 2. **Credentials are never reachable.** `vibe_core::path_guard` (`~/.ssh`,
//!    `~/.aws`, `~/.vibecli`, `id_rsa`, `daemon.token`, …) is applied to every
//!    outside path and is not overridable — not by `read_outside`, not by an
//!    `allow_root` pointing straight at it. A policy toggle that could hand over
//!    `~/.ssh/id_rsa` is not a policy, it is a vulnerability.
//! 3. **Deny beats allow.** `deny_roots` is checked before `allow_roots`, so a
//!    broad allow with a narrow deny does what it looks like.
//!
//! Note the credential deny-list is deliberately *not* applied inside the
//! workspace: `DENIED_FILENAMES` includes `config.json`, which is an ordinary
//! project file. Widening it inward would break real projects to protect files
//! the user already owns and pointed us at.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The kind of access being requested. Read and write are separate axes because
/// "let it look at my other repo" and "let it edit my other repo" are different
/// decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

impl Access {
    fn verb(self) -> &'static str {
        match self {
            Access::Read => "read",
            Access::Write => "write",
        }
    }
}

/// Fine-grained permissions for paths outside the workspace.
///
/// `Default` denies everything, which is exactly today's behaviour — so an
/// executor built without a policy behaves as it always has.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxPolicy {
    /// Read files outside the workspace.
    pub read_outside: bool,
    /// Create or modify files outside the workspace.
    pub write_outside: bool,
    /// Let shell commands act outside the workspace.
    ///
    /// Unlike the file tools, `bash` is not path-jailed — it always runs with
    /// the workspace as its cwd but the shell can reach anything the OS allows,
    /// which is true in ordinary Agent mode too. So this axis is enforced the
    /// only way it can be: when **off**, commands run under OS-level
    /// sandboxing (`bwrap`+Landlock on Linux, `sandbox-exec` on macOS,
    /// AppContainer on Windows) which confines them to the workspace. When on,
    /// they run unconfined, as they do today.
    pub exec_outside: bool,
    /// Reach the network (web search, URL fetch, outbound HTTP from commands).
    pub network: bool,
    /// When non-empty, outside access is confined to these roots. Empty means
    /// "anywhere the axis flags and the deny rules allow".
    pub allow_roots: Vec<PathBuf>,
    /// Never reachable, even when an axis is enabled or an `allow_root` covers
    /// it. Checked first.
    pub deny_roots: Vec<PathBuf>,
}

impl SandboxPolicy {
    /// The jail-only policy: nothing outside the workspace. Same as `default()`,
    /// named for call sites where the intent should be legible.
    pub fn locked() -> Self {
        Self::default()
    }

    /// True when this policy grants nothing — the executor can then skip the
    /// outside-path checks entirely and behave exactly as before.
    pub fn is_locked(&self) -> bool {
        !self.read_outside && !self.write_outside && !self.exec_outside && !self.network
    }

    /// Whether `path` (already canonical, already known to be outside the
    /// workspace) may be accessed. `Err` carries a message naming the reason,
    /// because "denied" with no cause is unactionable for the model and the user.
    pub fn allows(&self, path: &Path, access: Access) -> Result<(), String> {
        // 1. Credentials, always, regardless of configuration.
        if let Some(reason) = sensitive_reason(path) {
            return Err(format!(
                "Blocked: '{}' is a protected credential path ({reason}). \
                 This cannot be enabled by any sandbox setting.",
                path.display()
            ));
        }

        // 2. Explicit denies beat everything below.
        if let Some(root) = self.deny_roots.iter().find(|r| path.starts_with(r)) {
            return Err(format!(
                "Blocked: '{}' is under a denied root ('{}').",
                path.display(),
                root.display()
            ));
        }

        // 3. The axis for this operation.
        let axis_open = match access {
            Access::Read => self.read_outside,
            Access::Write => self.write_outside,
        };
        if !axis_open {
            return Err(format!(
                "Blocked: '{}' is outside the workspace and sandbox {} access is off.",
                path.display(),
                access.verb()
            ));
        }

        // 4. When an allowlist exists, outside access is confined to it.
        if !self.allow_roots.is_empty() && !self.allow_roots.iter().any(|r| path.starts_with(r)) {
            return Err(format!(
                "Blocked: '{}' is outside the workspace and not under any allowed root.",
                path.display()
            ));
        }

        Ok(())
    }
}

/// Why `path` is considered sensitive, or `None`.
///
/// Delegates to the canonical deny-list in `vibe_core::path_guard` so the
/// daemon and the desktop clients cannot drift on what counts as a credential.
fn sensitive_reason(path: &Path) -> Option<String> {
    vibe_core::path_guard::reject_sensitive_path(&path.to_string_lossy()).err()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> SandboxPolicy {
        SandboxPolicy {
            read_outside: true,
            write_outside: true,
            exec_outside: true,
            network: true,
            ..Default::default()
        }
    }

    #[test]
    fn default_denies_every_axis() {
        let p = SandboxPolicy::default();
        assert!(p.is_locked());
        assert!(p.allows(Path::new("/tmp/x"), Access::Read).is_err());
        assert!(p.allows(Path::new("/tmp/x"), Access::Write).is_err());
    }

    #[test]
    fn read_and_write_are_separate_axes() {
        let p = SandboxPolicy {
            read_outside: true,
            ..Default::default()
        };
        assert!(p.allows(Path::new("/tmp/other/x.rs"), Access::Read).is_ok());
        let err = p
            .allows(Path::new("/tmp/other/x.rs"), Access::Write)
            .unwrap_err();
        assert!(err.contains("write"), "{err}");
    }

    /// The rule that must survive every future refactor: no configuration
    /// reaches a credential path.
    #[test]
    fn credentials_are_never_reachable_however_configured() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/Users/example"));
        let secrets = [
            home.join(".ssh").join("id_rsa"),
            home.join(".aws").join("credentials"),
            home.join(".vibecli").join("daemon.token"),
        ];
        // Maximally permissive: every axis on, and the secret's own directory
        // explicitly allow-listed.
        for secret in &secrets {
            let p = SandboxPolicy {
                allow_roots: vec![secret.parent().unwrap().to_path_buf()],
                ..open()
            };
            for access in [Access::Read, Access::Write] {
                let err = p.allows(secret, access).unwrap_err();
                assert!(
                    err.contains("credential"),
                    "{} should be blocked as a credential, got: {err}",
                    secret.display()
                );
            }
        }
    }

    #[test]
    fn deny_roots_beat_allow_roots() {
        let p = SandboxPolicy {
            allow_roots: vec![PathBuf::from("/work")],
            deny_roots: vec![PathBuf::from("/work/secret")],
            ..open()
        };
        assert!(p.allows(Path::new("/work/ok.rs"), Access::Read).is_ok());
        let err = p
            .allows(Path::new("/work/secret/x"), Access::Read)
            .unwrap_err();
        assert!(err.contains("denied root"), "{err}");
    }

    #[test]
    fn an_allowlist_confines_outside_access() {
        let p = SandboxPolicy {
            allow_roots: vec![PathBuf::from("/work")],
            ..open()
        };
        assert!(p.allows(Path::new("/work/a"), Access::Read).is_ok());
        let err = p
            .allows(Path::new("/elsewhere/a"), Access::Read)
            .unwrap_err();
        assert!(err.contains("not under any allowed root"), "{err}");
    }

    #[test]
    fn empty_allowlist_means_anywhere_not_otherwise_denied() {
        let p = open();
        assert!(p.allows(Path::new("/elsewhere/a"), Access::Read).is_ok());
    }

    /// `exec_outside` is enforced by OS-level confinement, not by a path check
    /// (the shell is not jailed), so the policy's job is only to carry the bit.
    #[test]
    fn exec_is_a_separate_axis_from_file_access() {
        let p = SandboxPolicy {
            read_outside: true,
            write_outside: true,
            ..Default::default()
        };
        assert!(!p.exec_outside, "file access must not imply command reach");
        assert!(!p.is_locked(), "granting file access unlocks the policy");
    }

    #[test]
    fn is_locked_tracks_the_axes_not_the_roots() {
        let mut p = SandboxPolicy {
            allow_roots: vec![PathBuf::from("/work")],
            ..Default::default()
        };
        assert!(p.is_locked(), "roots alone grant nothing");
        p.read_outside = true;
        assert!(!p.is_locked());
    }

    /// Round-trips over the wire as part of the agent request.
    #[test]
    fn serde_round_trips_and_defaults_missing_fields() {
        let json = r#"{"read_outside":true,"allow_roots":["/work"]}"#;
        let p: SandboxPolicy = serde_json::from_str(json).unwrap();
        assert!(p.read_outside);
        assert!(!p.write_outside, "absent fields must default to denied");
        assert_eq!(p.allow_roots, vec![PathBuf::from("/work")]);
        let back = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<SandboxPolicy>(&back).unwrap(), p);
    }
}
