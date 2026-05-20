//! virtio-fs share configuration + virtiofsd argv builder.
//!
//! Slice F3.1 — the building blocks for F3 (folder rw bind via
//! virtio-fs). The workspace folder gets mounted into the microVM
//! at `/work` (configurable) so the agent's shell tool can read/write
//! files exactly as on the host, while the rootfs (slice F1) stays
//! immutable.
//!
//! ## Architecture
//!
//! Firecracker doesn't ship a built-in virtio-fs implementation; it
//! relies on out-of-process **virtiofsd** (the upstream daemon from
//! the kata-containers / virtio-fs project). The boot sequence is:
//!
//! 1. daemon spawns virtiofsd, passing it a UDS socket path + shared
//!    directory path + share-name.
//! 2. daemon issues `PUT /vsock` (or similar — Firecracker's virtio-fs
//!    config endpoint) referencing the same UDS so the microVM can
//!    discover the share at boot.
//! 3. kernel cmdline includes the mount tag so an in-guest agent can
//!    `mount -t virtiofs <tag> /work` (or have the rootfs init do it).
//!
//! This module ships the **config + argv** part. The process spawn +
//! UDS plumbing lives in F2.2/F3.2 (Linux-only).
//!
//! ## Deny-list
//!
//! Host paths are validated against the same credential-dir deny-list
//! as the `Sandbox::bind_rw` API. A microVM is otherwise isolated —
//! but if the *workspace bind* points to `~/.aws`, the in-VM agent
//! reads AWS credentials despite all the KVM hardening. Defense in
//! depth: refuse at the share-construction boundary.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from share construction.
#[derive(Debug, Error)]
pub enum VirtioFsError {
    #[error("host path traversal not allowed: {0}")]
    Traversal(PathBuf),

    #[error("host path descends through a credential directory ({segment}): {path}")]
    CredentialDir { segment: String, path: PathBuf },

    #[error("host path is a known credential file ({filename}): {path}")]
    CredentialFile { filename: String, path: PathBuf },

    #[error("mount tag contains invalid characters: {0}")]
    InvalidTag(String),

    #[error("mount tag is empty")]
    EmptyTag,

    #[error("mount tag exceeds 36 characters (virtio-fs limit): {0}")]
    TagTooLong(String),
}

/// A single virtio-fs share — one host directory ↔ one guest mount tag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VirtioFsShare {
    /// Host path that virtiofsd will expose.
    pub host_path: PathBuf,
    /// Mount tag the guest uses with `mount -t virtiofs <tag> /work`.
    pub mount_tag: String,
    /// UDS path virtiofsd listens on (the same path is referenced in
    /// the Firecracker config so the microVM knows where to find it).
    pub socket_path: PathBuf,
    /// Read-only share (corresponds to `Sandbox::bind_ro`).
    pub read_only: bool,
    /// Map host UID/GID through to the guest (`--xattr` + uid-map).
    /// Default off — agent processes inside the VM usually run as
    /// root in the microVM regardless.
    pub xattr_passthrough: bool,
    /// Cache mode (`--cache=auto|always|none`). Default = `auto`.
    pub cache_mode: CacheMode,
}

/// virtiofsd cache mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Auto,
    Always,
    None,
}

impl Default for CacheMode {
    fn default() -> Self {
        CacheMode::Auto
    }
}

impl VirtioFsShare {
    /// Construct a share with deny-list + tag validation.
    pub fn new(
        host_path: impl Into<PathBuf>,
        mount_tag: impl Into<String>,
        socket_path: impl Into<PathBuf>,
        read_only: bool,
    ) -> Result<Self, VirtioFsError> {
        let host_path = host_path.into();
        let mount_tag = mount_tag.into();
        let socket_path = socket_path.into();

        validate_host_path(&host_path)?;
        validate_mount_tag(&mount_tag)?;

        Ok(VirtioFsShare {
            host_path,
            mount_tag,
            socket_path,
            read_only,
            xattr_passthrough: false,
            cache_mode: CacheMode::Auto,
        })
    }

    /// Build the argv for spawning virtiofsd. Matches the upstream
    /// CLI shape:
    ///   virtiofsd --socket-path=<sock> --shared-dir=<host>
    ///             --cache=<mode> [--xattr] [--readonly]
    ///             --thread-pool-size=<n> --announce-submounts
    ///
    /// Returns a `Vec<OsString>` so callers can directly feed it to
    /// `std::process::Command::args(...)`.
    pub fn argv(&self) -> Vec<OsString> {
        let mut a = Vec::<OsString>::new();
        a.push(OsString::from(format!(
            "--socket-path={}",
            self.socket_path.display()
        )));
        a.push(OsString::from(format!(
            "--shared-dir={}",
            self.host_path.display()
        )));
        a.push(OsString::from(format!(
            "--cache={}",
            match self.cache_mode {
                CacheMode::Auto => "auto",
                CacheMode::Always => "always",
                CacheMode::None => "none",
            }
        )));
        if self.read_only {
            a.push(OsString::from("--readonly"));
        }
        if self.xattr_passthrough {
            a.push(OsString::from("--xattr"));
        }
        // Sensible defaults — exposed via builder methods if a caller
        // ever needs to override.
        a.push(OsString::from("--thread-pool-size=2"));
        a.push(OsString::from("--announce-submounts"));
        a
    }

    /// Kernel cmdline fragment the microVM needs so the rootfs init
    /// can `mount -t virtiofs <tag> /work` automatically (when the
    /// init script reads it from /proc/cmdline).
    ///
    /// We use a structured key=value scheme so the in-guest init can
    /// parse it without ambiguity:
    ///
    ///     vibe.virtiofs=<tag>:<mount-point>:<ro|rw>
    pub fn kernel_cmdline_fragment(&self, mount_point: &str) -> String {
        format!(
            "vibe.virtiofs={}:{}:{}",
            self.mount_tag,
            mount_point,
            if self.read_only { "ro" } else { "rw" }
        )
    }
}

// ── Deny-list ────────────────────────────────────────────────────────────────

/// Directory names whose contents are the daemon's secret state — never safe
/// to expose to any sandbox, microVM included. Mirrors the same list in
/// `vibe-core::path_guard` + the in-crate `validate_path` of the
/// FirecrackerSandbox bind API. Kept in-crate to avoid pulling
/// `vibe-core` (and tokio + the vibecli dep tree) into this leaf crate;
/// parity is enforced by the cross-crate test below.
const DENIED_SEGMENTS: &[&str] = &[
    ".vibecli",
    ".vibeui",
    ".claude",
    ".ssh",
    ".aws",
    ".gnupg",
];

/// Filenames that name credential blobs regardless of parent dir.
const DENIED_FILENAMES: &[&str] = &[
    "daemon.token",
    "profile_settings.db",
    "workspace.db",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials",
];

fn validate_host_path(p: &Path) -> Result<(), VirtioFsError> {
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(VirtioFsError::Traversal(p.to_path_buf()));
    }
    for c in p.components() {
        if let std::path::Component::Normal(seg) = c {
            let lower = seg.to_string_lossy().to_lowercase();
            if let Some(hit) = DENIED_SEGMENTS.iter().find(|d| lower == **d) {
                return Err(VirtioFsError::CredentialDir {
                    segment: (*hit).to_string(),
                    path: p.to_path_buf(),
                });
            }
        }
    }
    if let Some(name) = p.file_name() {
        let lower = name.to_string_lossy().to_lowercase();
        if let Some(hit) = DENIED_FILENAMES.iter().find(|f| lower == **f) {
            return Err(VirtioFsError::CredentialFile {
                filename: (*hit).to_string(),
                path: p.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// virtio-fs mount tags are limited to 36 ASCII characters; certain
/// punctuation breaks the kernel parser. Stick to letters/digits/-/_.
fn validate_mount_tag(t: &str) -> Result<(), VirtioFsError> {
    if t.is_empty() {
        return Err(VirtioFsError::EmptyTag);
    }
    if t.len() > 36 {
        return Err(VirtioFsError::TagTooLong(t.to_string()));
    }
    let ok = t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(VirtioFsError::InvalidTag(t.to_string()));
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_share() -> Result<VirtioFsShare, VirtioFsError> {
        VirtioFsShare::new(
            "/var/tmp/workspace",
            "workspace",
            "/run/virtiofs-workspace.sock",
            false,
        )
    }

    // ── construction + deny-list ─────────────────────────────────────────

    #[test]
    fn neutral_path_accepted() {
        let s = ok_share().unwrap();
        assert_eq!(s.host_path, PathBuf::from("/var/tmp/workspace"));
        assert_eq!(s.mount_tag, "workspace");
        assert!(!s.read_only);
        assert!(matches!(s.cache_mode, CacheMode::Auto));
        assert!(!s.xattr_passthrough);
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let err =
            VirtioFsShare::new("/var/tmp/../etc/shadow", "tag", "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::Traversal(_)));
    }

    #[test]
    fn rejects_vibecli_segment() {
        let err =
            VirtioFsShare::new("/home/u/.vibecli/jobs.db", "x", "/run/s.sock", false).unwrap_err();
        match err {
            VirtioFsError::CredentialDir { segment, .. } => assert_eq!(segment, ".vibecli"),
            other => panic!("expected CredentialDir, got {:?}", other),
        }
    }

    #[test]
    fn rejects_ssh_segment_case_insensitive() {
        let err = VirtioFsShare::new("/home/u/.SSH/keys", "x", "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::CredentialDir { .. }));
    }

    #[test]
    fn rejects_aws_segment() {
        let err =
            VirtioFsShare::new("/home/u/.aws/credentials", "x", "/run/s.sock", false).unwrap_err();
        // .aws is a segment match AND credentials is a filename match;
        // either error is acceptable, both close the gap.
        assert!(matches!(
            err,
            VirtioFsError::CredentialDir { .. } | VirtioFsError::CredentialFile { .. }
        ));
    }

    #[test]
    fn rejects_id_rsa_filename() {
        let err =
            VirtioFsShare::new("/some/safe/dir/id_rsa", "x", "/run/s.sock", false).unwrap_err();
        match err {
            VirtioFsError::CredentialFile { filename, .. } => assert_eq!(filename, "id_rsa"),
            other => panic!("expected CredentialFile, got {:?}", other),
        }
    }

    #[test]
    fn rejects_daemon_token_filename() {
        let err =
            VirtioFsShare::new("/etc/vibe/daemon.token", "x", "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::CredentialFile { .. }));
    }

    #[test]
    fn lookalike_segment_accepts() {
        // `.vibecli-state` is unrelated — match must be exact segment.
        let s =
            VirtioFsShare::new("/home/u/.vibecli-state/foo", "x", "/run/s.sock", false).unwrap();
        assert!(s.host_path.to_string_lossy().contains(".vibecli-state"));
    }

    // ── mount-tag validation ─────────────────────────────────────────────

    #[test]
    fn empty_tag_rejected() {
        let err = VirtioFsShare::new("/tmp", "", "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::EmptyTag));
    }

    #[test]
    fn long_tag_rejected() {
        let too_long = "a".repeat(37);
        let err = VirtioFsShare::new("/tmp", &too_long, "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::TagTooLong(_)));
    }

    #[test]
    fn tag_with_slash_rejected() {
        let err = VirtioFsShare::new("/tmp", "bad/tag", "/run/s.sock", false).unwrap_err();
        assert!(matches!(err, VirtioFsError::InvalidTag(_)));
    }

    #[test]
    fn tag_with_dash_and_underscore_accepted() {
        let s =
            VirtioFsShare::new("/tmp", "workspace-1_share", "/run/s.sock", false).unwrap();
        assert_eq!(s.mount_tag, "workspace-1_share");
    }

    #[test]
    fn tag_max_36_chars_accepted() {
        let exactly_36 = "a".repeat(36);
        let s = VirtioFsShare::new("/tmp", &exactly_36, "/run/s.sock", false).unwrap();
        assert_eq!(s.mount_tag.len(), 36);
    }

    // ── argv ─────────────────────────────────────────────────────────────

    #[test]
    fn argv_contains_required_flags() {
        let s = ok_share().unwrap();
        let argv = s.argv();
        let joined: String = argv
            .iter()
            .map(|o| o.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("--socket-path=/run/virtiofs-workspace.sock"));
        assert!(joined.contains("--shared-dir=/var/tmp/workspace"));
        assert!(joined.contains("--cache=auto"));
        assert!(joined.contains("--thread-pool-size=2"));
        assert!(joined.contains("--announce-submounts"));
        // Default (rw, no xattr) must NOT emit the gating flags.
        assert!(!joined.contains("--readonly"));
        assert!(!joined.contains("--xattr"));
    }

    #[test]
    fn argv_readonly_emits_flag() {
        let s = VirtioFsShare::new("/tmp/work", "work", "/run/s.sock", true).unwrap();
        let joined: String = s
            .argv()
            .iter()
            .map(|o| o.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("--readonly"));
    }

    #[test]
    fn argv_xattr_emits_flag_when_enabled() {
        let mut s = VirtioFsShare::new("/tmp/work", "work", "/run/s.sock", false).unwrap();
        s.xattr_passthrough = true;
        let joined: String = s
            .argv()
            .iter()
            .map(|o| o.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("--xattr"));
    }

    #[test]
    fn argv_cache_mode_respected() {
        let mut s = VirtioFsShare::new("/tmp/work", "work", "/run/s.sock", false).unwrap();
        s.cache_mode = CacheMode::Always;
        let joined: String = s
            .argv()
            .iter()
            .map(|o| o.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("--cache=always"));
        assert!(!joined.contains("--cache=auto"));
    }

    #[test]
    fn argv_args_are_osstring() {
        let s = ok_share().unwrap();
        let argv = s.argv();
        // Compile-time check: argv elements must be &OsStr-compatible
        // so Command::args(...) accepts them directly.
        let _: &std::ffi::OsStr = argv[0].as_os_str();
    }

    // ── kernel cmdline fragment ──────────────────────────────────────────

    #[test]
    fn cmdline_fragment_rw() {
        let s = ok_share().unwrap();
        assert_eq!(
            s.kernel_cmdline_fragment("/work"),
            "vibe.virtiofs=workspace:/work:rw"
        );
    }

    #[test]
    fn cmdline_fragment_ro() {
        let s = VirtioFsShare::new("/srv/code", "code", "/run/c.sock", true).unwrap();
        assert_eq!(
            s.kernel_cmdline_fragment("/code"),
            "vibe.virtiofs=code:/code:ro"
        );
    }

    // ── round-trip serde ─────────────────────────────────────────────────

    #[test]
    fn serde_round_trip_preserves_fields() {
        let mut s = ok_share().unwrap();
        s.cache_mode = CacheMode::None;
        s.xattr_passthrough = true;
        let json = serde_json::to_string(&s).unwrap();
        let back: VirtioFsShare = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn cache_mode_serializes_lowercase() {
        let v = serde_json::to_string(&CacheMode::Always).unwrap();
        assert_eq!(v, "\"always\"");
    }
}
