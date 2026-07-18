//! Linux Tier-0 backend: bwrap + Landlock + seccomp.
//!
//! Three layers, composed:
//! 1. bwrap sets up namespaces and bind-mounts the user's folder.
//! 2. Landlock layers FS-access rules inside the namespace (kernel ≥ 5.13).
//! 3. seccomp filters syscalls to a curated allow-list.
//!
//! See `docs/design/sandbox-tiers/01-native-tier.md`.
//!
//! NOTE: Landlock + seccomp are wired via a small entry shim binary that
//! runs as the first process inside the bwrap'd namespace, applies both
//! filters, then `execve`s the target. v1 ships the bwrap layer; the shim
//! lands in slice N1.2 (Landlock) and N1.3 (seccomp).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use vibe_sandbox::{
    BindMode, EnvPolicy, NetPolicy, ResourceLimits, Result, Sandbox, SandboxError, SandboxTier,
};

#[derive(Debug, Default)]
pub struct LinuxSandbox {
    rw: Vec<(PathBuf, PathBuf)>,
    ro: Vec<(PathBuf, PathBuf)>,
    unshare_net: bool,
    broker_socket: Option<PathBuf>,
    env: EnvPolicy,
    limits: ResourceLimits,
}

impl LinuxSandbox {
    pub fn new() -> Result<Self> {
        Ok(LinuxSandbox {
            unshare_net: true,
            ..Default::default()
        })
    }

    /// Build the bwrap argv list for the configured policy.
    /// Public for testability — called by `spawn` to assemble the command.
    pub fn build_bwrap_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "--unshare-pid".into(),
            "--unshare-ipc".into(),
            "--unshare-uts".into(),
            "--unshare-cgroup-try".into(),
            "--die-with-parent".into(),
            "--new-session".into(),
            "--proc".into(),
            "/proc".into(),
            "--dev".into(),
            "/dev".into(),
            "--tmpfs".into(),
            "/tmp".into(),
        ];
        if self.unshare_net {
            args.push("--unshare-net".into());
        }
        for (host, guest) in &self.rw {
            args.push("--bind".into());
            args.push(host.to_string_lossy().into_owned());
            args.push(guest.to_string_lossy().into_owned());
        }
        for (host, guest) in &self.ro {
            args.push("--ro-bind".into());
            args.push(host.to_string_lossy().into_owned());
            args.push(guest.to_string_lossy().into_owned());
        }
        if let Some(sock) = &self.broker_socket {
            // bind the broker socket into the namespace at a known path
            args.push("--bind".into());
            args.push(sock.to_string_lossy().into_owned());
            args.push("/run/vibe-broker.sock".into());
        }
        // Standard read-only system mounts. Tier-0 design assumes the host
        // has /usr, /lib, /lib64, /bin, /sbin, /etc available.
        for ro_default in ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"] {
            if Path::new(ro_default).exists() {
                args.push("--ro-bind".into());
                args.push(ro_default.into());
                args.push(ro_default.into());
            }
        }
        args
    }
}

impl Sandbox for LinuxSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        validate_path(host)?;
        validate_path(guest)?;
        self.rw.push((host.to_owned(), guest.to_owned()));
        Ok(())
    }

    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        validate_path(host)?;
        validate_path(guest)?;
        self.ro.push((host.to_owned(), guest.to_owned()));
        Ok(())
    }

    fn env(&mut self, policy: EnvPolicy) {
        self.env = policy;
    }

    fn limits(&mut self, limits: ResourceLimits) {
        self.limits = limits;
    }

    fn network(&mut self, policy: NetPolicy) {
        match policy {
            NetPolicy::None => {
                self.unshare_net = true;
                self.broker_socket = None;
            }
            NetPolicy::Brokered { socket, .. } => {
                self.unshare_net = true;
                self.broker_socket = Some(socket);
            }
            NetPolicy::Direct => {
                self.unshare_net = false;
                self.broker_socket = None;
            }
        }
    }

    fn spawn(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Child> {
        let bwrap_args = self.build_bwrap_args();
        let mut c = Command::new("bwrap");
        for a in &bwrap_args {
            c.arg(a);
        }
        c.arg("--").arg(cmd).args(args);
        c.stdout(Stdio::piped()).stderr(Stdio::piped());
        c.spawn().map_err(SandboxError::Io)
    }

    fn tier(&self) -> SandboxTier {
        SandboxTier::Native
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

/// Directory names whose contents are the daemon's secret state — never safe
/// to expose to a sandboxed process. A future caller mistakenly passing
/// `~/.vibecli` (the encrypted ProfileStore live there) or a workspace's
/// `.vibecli/` (WorkspaceStore + jobs.db) would let arbitrary sandboxed
/// code read the bearer token, API keys for every configured provider, OAuth
/// refresh tokens, and the workspace job/recap history. See
/// `docs/security/threat-model.md` §7 item #11 (DREAD 7.2).
const DENIED_SEGMENTS: &[&str] = &[".vibecli", ".vibecoder", ".claude"];

/// Specific filenames that name credential blobs, regardless of parent dir.
/// `daemon.token` is written by `vibecli serve`; the others are sqlx databases
/// holding encrypted state.
const DENIED_FILENAMES: &[&str] = &["daemon.token", "profile_settings.db", "workspace.db"];

fn validate_path(p: &Path) -> Result<()> {
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(SandboxError::Setup(format!(
            "path traversal not allowed in sandbox path: {}",
            p.display()
        )));
    }
    // Reject paths that descend through a VibeCody secret-state directory.
    // Segment-match, not prefix-match — `foo/.vibecli-docs/bar` is unrelated
    // and stays legal; `~/work/.vibecli/jobs.db` is denied.
    for c in p.components() {
        if let std::path::Component::Normal(seg) = c {
            if let Some(seg) = seg.to_str() {
                if DENIED_SEGMENTS.contains(&seg) {
                    return Err(SandboxError::Setup(format!(
                        "refuses to bind a VibeCody secret-state directory into the sandbox: {} \
                         (contains a '{seg}' segment — would expose the daemon's keychain to \
                         sandboxed code)",
                        p.display()
                    )));
                }
            }
        }
    }
    if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
        if DENIED_FILENAMES.contains(&name) {
            return Err(SandboxError::Setup(format!(
                "refuses to bind a VibeCody credential file into the sandbox: {} \
                 (file name '{name}' is a known credential blob)",
                p.display()
            )));
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn bind_with_mode(
    sandbox: &mut LinuxSandbox,
    host: &Path,
    guest: &Path,
    mode: BindMode,
) -> Result<()> {
    match mode {
        BindMode::Rw => sandbox.bind_rw(host, guest),
        BindMode::Ro => sandbox.bind_ro(host, guest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_includes_default_unshares() {
        let sb = LinuxSandbox::new().unwrap();
        let args = sb.build_bwrap_args();
        assert!(args.iter().any(|a| a == "--unshare-pid"));
        assert!(args.iter().any(|a| a == "--unshare-net"));
        assert!(args.iter().any(|a| a == "--die-with-parent"));
    }

    #[test]
    fn bind_rw_appears_in_args() {
        let mut sb = LinuxSandbox::new().unwrap();
        sb.bind_rw(Path::new("/host/work"), Path::new("/work"))
            .unwrap();
        let args = sb.build_bwrap_args();
        let mut iter = args.iter();
        let mut found = false;
        while let Some(a) = iter.next() {
            if a == "--bind" {
                if iter.next().map(|s| s.as_str()) == Some("/host/work")
                    && iter.next().map(|s| s.as_str()) == Some("/work")
                {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "expected --bind /host/work /work in {:?}", args);
    }

    #[test]
    fn bind_ro_appears_in_args() {
        let mut sb = LinuxSandbox::new().unwrap();
        sb.bind_ro(Path::new("/host/ro"), Path::new("/ro")).unwrap();
        let args = sb.build_bwrap_args();
        assert!(args
            .windows(3)
            .any(|w| { w[0] == "--ro-bind" && w[1] == "/host/ro" && w[2] == "/ro" }));
    }

    #[test]
    fn direct_network_disables_unshare_net() {
        let mut sb = LinuxSandbox::new().unwrap();
        sb.network(NetPolicy::Direct);
        let args = sb.build_bwrap_args();
        assert!(!args.iter().any(|a| a == "--unshare-net"));
    }

    #[test]
    fn brokered_network_binds_socket_path() {
        let mut sb = LinuxSandbox::new().unwrap();
        sb.network(NetPolicy::Brokered {
            socket: PathBuf::from("/run/vibe-broker.sock"),
            policy_id: "skill:test".into(),
        });
        let args = sb.build_bwrap_args();
        assert!(args.iter().any(|a| a == "/run/vibe-broker.sock"));
        assert!(args.iter().any(|a| a == "--unshare-net"));
    }

    #[test]
    fn validate_path_rejects_traversal() {
        assert!(validate_path(Path::new("/tmp/../etc")).is_err());
    }

    // ── DREAD #11 regression guards ─────────────────────────────────────

    #[test]
    fn validate_path_rejects_user_vibecli_state_dir() {
        let err = validate_path(Path::new("/home/alice/.vibecli")).unwrap_err();
        assert!(format!("{err}").contains(".vibecli"));
    }

    #[test]
    fn validate_path_rejects_workspace_vibecli_state_dir() {
        let err = validate_path(Path::new("/home/alice/code/myrepo/.vibecli")).unwrap_err();
        assert!(format!("{err}").contains(".vibecli"));
    }

    #[test]
    fn validate_path_rejects_path_descending_through_vibecli() {
        // Even if the leaf isn't the .vibecli dir itself, descending through
        // it is denied — that's how `~/.vibecli/profile_settings.db` would
        // sneak in past a leaf-only filter.
        let err = validate_path(Path::new("/home/alice/.vibecli/profile_settings.db")).unwrap_err();
        assert!(format!("{err}").contains(".vibecli"));
    }

    #[test]
    fn validate_path_rejects_user_vibecoder_state_dir() {
        let err = validate_path(Path::new("/home/alice/.vibecoder")).unwrap_err();
        assert!(format!("{err}").contains(".vibecoder"));
    }

    #[test]
    fn validate_path_rejects_daemon_token_filename() {
        // Even if a future caller built a path that doesn't go through a
        // `.vibecli` dir (e.g. user symlinked the token elsewhere), the
        // filename itself is denied.
        let err = validate_path(Path::new("/tmp/exported/daemon.token")).unwrap_err();
        assert!(format!("{err}").contains("daemon.token"));
    }

    #[test]
    fn validate_path_rejects_profile_settings_db() {
        let err = validate_path(Path::new("/tmp/backup/profile_settings.db")).unwrap_err();
        assert!(format!("{err}").contains("profile_settings.db"));
    }

    #[test]
    fn validate_path_allows_lookalike_names() {
        // Segment match, not prefix match — these are *not* VibeCody state
        // dirs and must remain legal sandbox binds.
        assert!(validate_path(Path::new("/home/alice/code/.vibecli-docs/notes.md")).is_ok());
        assert!(validate_path(Path::new("/home/alice/vibecli/work")).is_ok());
        assert!(validate_path(Path::new("/home/alice/code/myproject/.git")).is_ok());
    }

    #[test]
    fn bind_rw_rejects_vibecli_state_dir() {
        // End-to-end: the public API must surface the validation error.
        let mut sb = LinuxSandbox::new().unwrap();
        let err = sb
            .bind_rw(Path::new("/home/alice/.vibecli"), Path::new("/work"))
            .unwrap_err();
        assert!(format!("{err}").contains(".vibecli"));
    }

    #[test]
    fn tier_is_native() {
        let sb = LinuxSandbox::new().unwrap();
        assert_eq!(sb.tier(), SandboxTier::Native);
    }
}
