//! Tier-3 (Firecracker microVM) Sandbox impl for VibeCody.
//!
//! See `docs/design/sandbox-tiers/03-firecracker-tier.md` for the full
//! design. This crate is Linux-only — on any other OS the public
//! constructor returns `SandboxError::TierUnsupported`, and the
//! daemon's tier-selection layer transparently downgrades to Tier-0
//! per `docs/design/sandbox-tiers/README.md` §"Tier downgrade emits
//! a structured warning event."
//!
//! ## Sequencing
//!
//! This slice (T3.1.A) ships the **state-tracking skeleton**:
//!
//! * Public `FirecrackerSandbox` type that implements the
//!   [`Sandbox`] trait by accumulating bind / env / limits / network
//!   state in memory.
//! * `tier()` returns [`SandboxTier::Firecracker`] so the daemon's
//!   telemetry + recap pipeline tags microVM workloads correctly
//!   *now*, before the runtime is live.
//! * `spawn()` returns [`SandboxError::NotSupported`] with a
//!   structured message identifying T3.1.B as the gating slice.
//!   This keeps callers from accidentally running un-isolated when
//!   they ask for Firecracker — the contract is fail-closed.
//! * Cfg-gated to Linux: on macOS / Windows the public constructor
//!   refuses construction so a misconfigured `default_tier =
//!   "firecracker"` on those hosts can't silently fall back to a
//!   stub that pretends to be Firecracker.
//!
//! The next slice (T3.1.B) wires the real microVM lifecycle:
//! * BusyBox + bash rootfs builder
//! * `firecracker --api-sock …` spawn + PID 1 hand-off
//! * virtio-vsock to `vibe-broker` for network
//! * Resource limits → microVM kernel boot args + cgroups
//! * `Child` returned from `spawn` is the host-side `firecracker(1)`
//!   process; the in-guest workload talks back to the daemon via
//!   the vsock control plane.

pub mod api;
pub mod rootfs;
pub mod virtiofs;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Child;

use vibe_sandbox::{
    BindMode, EnvPolicy, NetPolicy, ResourceLimits, Result, Sandbox, SandboxError, SandboxTier,
};

/// Tier-3 Firecracker sandbox.
///
/// State-tracking only in slice T3.1.A — see crate-level docs.
#[derive(Debug)]
pub struct FirecrackerSandbox {
    /// Read-write bind mounts (host → guest). Validated against the
    /// shared `vibe-core::path_guard` deny-list at bind time so a
    /// caller can't ask Firecracker to mount `~/.aws` even though
    /// the microVM is otherwise isolated — defense in depth.
    binds: Vec<(PathBuf, PathBuf, BindMode)>,
    env: EnvPolicy,
    limits: ResourceLimits,
    network: NetPolicy,
    /// Linux-only fields for the in-progress microVM lifecycle.
    /// Populated by builder methods in slice T3.1.B.
    #[cfg(target_os = "linux")]
    runtime: LinuxRuntimeState,
    /// Marker so `Drop` cleanup runs at the right place on all OSes.
    _platform_marker: PlatformMarker,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
struct LinuxRuntimeState {
    /// Path to the Firecracker binary. Defaults to PATH lookup of
    /// `firecracker` at spawn time.
    pub firecracker_bin: Option<PathBuf>,
    /// Path to the uncompressed Linux kernel image (vmlinux). T3.1.B
    /// will ship a minimal kernel as a build artifact; for now this
    /// is a user-supplied path with no default.
    pub kernel_image: Option<PathBuf>,
    /// Path to the rootfs ext4 image. T3.1.B will build a BusyBox +
    /// bash rootfs (~10 MB). For now user-supplied.
    pub rootfs_image: Option<PathBuf>,
    /// Unix socket where Firecracker exposes its REST API.
    /// Auto-generated under `$XDG_RUNTIME_DIR` at spawn time.
    pub api_socket: Option<PathBuf>,
    /// vsock CID for guest ↔ broker bridge. Default 3 (CID 2 is
    /// reserved for the host).
    pub vsock_cid: u32,
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Default)]
#[allow(dead_code)]
struct LinuxRuntimeState;

#[derive(Debug, Default)]
struct PlatformMarker;

impl FirecrackerSandbox {
    /// Construct a new Firecracker sandbox skeleton.
    ///
    /// Linux: returns a state-tracking sandbox; `spawn()` will refuse
    /// until slice T3.1.B wires the microVM.
    ///
    /// Other OSes: returns `SandboxError::TierUnsupported`.
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                binds: Vec::new(),
                env: EnvPolicy::Clear,
                limits: ResourceLimits::default(),
                network: NetPolicy::None,
                runtime: LinuxRuntimeState {
                    vsock_cid: 3,
                    ..Default::default()
                },
                _platform_marker: PlatformMarker,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(SandboxError::TierUnsupported {
                tier: SandboxTier::Firecracker,
            })
        }
    }

    /// Set the path to the Firecracker binary. When unset the
    /// runtime searches PATH at spawn time.
    #[cfg(target_os = "linux")]
    pub fn firecracker_bin(mut self, path: PathBuf) -> Self {
        self.runtime.firecracker_bin = Some(path);
        self
    }

    /// Set the uncompressed Linux kernel image (vmlinux) used to
    /// boot the microVM.
    #[cfg(target_os = "linux")]
    pub fn kernel_image(mut self, path: PathBuf) -> Self {
        self.runtime.kernel_image = Some(path);
        self
    }

    /// Set the rootfs ext4 image used as the guest root filesystem.
    #[cfg(target_os = "linux")]
    pub fn rootfs_image(mut self, path: PathBuf) -> Self {
        self.runtime.rootfs_image = Some(path);
        self
    }

    /// Read-only view of the current bind list. Tests + telemetry.
    pub fn binds(&self) -> &[(PathBuf, PathBuf, BindMode)] {
        &self.binds
    }
}

impl Sandbox for FirecrackerSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        validate_bind_path(host)?;
        self.binds
            .push((host.to_owned(), guest.to_owned(), BindMode::Rw));
        Ok(())
    }

    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        validate_bind_path(host)?;
        self.binds
            .push((host.to_owned(), guest.to_owned(), BindMode::Ro));
        Ok(())
    }

    fn env(&mut self, policy: EnvPolicy) {
        self.env = policy;
    }

    fn limits(&mut self, limits: ResourceLimits) {
        self.limits = limits;
    }

    fn network(&mut self, policy: NetPolicy) {
        self.network = policy;
    }

    fn spawn(&self, _cmd: &OsStr, _args: &[&OsStr]) -> Result<Child> {
        // Fail-closed: until T3.1.B wires the real microVM lifecycle,
        // a caller asking for Firecracker must NOT silently fall back
        // to running un-isolated on the host. Return the typed
        // NotSupported error so the daemon's tier-selection layer can
        // emit a `sandbox.downgrade` event or refuse the call
        // outright per policy.
        Err(SandboxError::NotSupported(
            "Firecracker microVM lifecycle is gated on slice T3.1.B \
             — see docs/design/sandbox-tiers/03-firecracker-tier.md",
        ))
    }

    fn tier(&self) -> SandboxTier {
        SandboxTier::Firecracker
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        // T3.1.B will tear down the microVM, unmount drives, close
        // the API socket, free the vsock CID, etc. Slice T3.1.A has
        // no live runtime — shutdown is a no-op.
        Ok(())
    }
}

/// Reject bind sources that traverse a credential directory. The
/// microVM is isolated from the host, but the bind itself happens on
/// the host before the VM starts — a caller mounting `~/.aws` into
/// the guest would leak credentials into the guest's filesystem.
/// This guard mirrors the Tier-0 native backends.
fn validate_bind_path(host: &Path) -> Result<()> {
    // We re-derive the deny-list here rather than depending on
    // `vibe-core` (which would pull tokio + every other vibecli
    // workspace dep into this otherwise lean crate). The list is
    // small, audited in `vibe-core`'s test suite, and the parity is
    // covered by `tests/firecracker_deny_list_parity.rs`.
    const DENIED_SEGMENTS: &[&str] = &[
        ".vibecli", ".vibeui", ".claude", ".ssh", ".aws", ".gnupg",
    ];
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
    for component in host.components() {
        if let std::path::Component::Normal(seg) = component {
            let s = seg.to_string_lossy();
            for denied in DENIED_SEGMENTS {
                if s.eq_ignore_ascii_case(denied) {
                    return Err(SandboxError::Setup(format!(
                        "refusing to bind credential directory '{}' into Firecracker guest",
                        denied
                    )));
                }
            }
        }
    }
    if let Some(name) = host.file_name().and_then(|n| n.to_str()) {
        for denied in DENIED_FILENAMES {
            if name.eq_ignore_ascii_case(denied) {
                return Err(SandboxError::Setup(format!(
                    "refusing to bind credential file '{}' into Firecracker guest",
                    denied
                )));
            }
        }
    }
    Ok(())
}

/// Public registration helper. `vibe-sandbox::select(Firecracker, …)`
/// returns this when the host supports it; the daemon's startup
/// banner advertises the tier.
pub fn is_supported() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    fn fresh() -> FirecrackerSandbox {
        FirecrackerSandbox::new().expect("Linux host must construct ok")
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn new_returns_firecracker_tier() {
        let sb = fresh();
        assert_eq!(sb.tier(), SandboxTier::Firecracker);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn spawn_fails_closed_until_t3_1_b_lands() {
        let sb = fresh();
        let cmd = std::ffi::OsString::from("/bin/echo");
        let arg = std::ffi::OsString::from("hi");
        let err = sb.spawn(&cmd, &[&arg]).expect_err("must refuse");
        match err {
            SandboxError::NotSupported(_) => {}
            other => panic!("expected NotSupported, got {:?}", other),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_rw_accumulates_in_order() {
        let mut sb = fresh();
        sb.bind_rw(Path::new("/tmp/a"), Path::new("/work/a")).unwrap();
        sb.bind_rw(Path::new("/tmp/b"), Path::new("/work/b")).unwrap();
        let binds = sb.binds();
        assert_eq!(binds.len(), 2);
        assert_eq!(binds[0].0, Path::new("/tmp/a"));
        assert!(matches!(binds[0].2, BindMode::Rw));
        assert!(matches!(binds[1].2, BindMode::Rw));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_ro_is_tagged_correctly() {
        let mut sb = fresh();
        sb.bind_ro(Path::new("/usr"), Path::new("/usr")).unwrap();
        assert!(matches!(sb.binds()[0].2, BindMode::Ro));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_refuses_credential_dir_segment() {
        let mut sb = fresh();
        let err = sb
            .bind_rw(Path::new("/Users/me/.aws"), Path::new("/work/.aws"))
            .expect_err("must refuse");
        match err {
            SandboxError::Setup(msg) => {
                assert!(msg.contains(".aws"), "got: {msg}");
            }
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_refuses_credential_filename() {
        let mut sb = fresh();
        let err = sb
            .bind_rw(Path::new("/tmp/id_rsa"), Path::new("/work/id_rsa"))
            .expect_err("must refuse");
        match err {
            SandboxError::Setup(msg) => assert!(msg.contains("id_rsa"), "got: {msg}"),
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_refuses_dot_ssh_segment_case_insensitive() {
        let mut sb = fresh();
        assert!(sb
            .bind_rw(Path::new("/Users/me/.SSH/keys"), Path::new("/work"))
            .is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bind_accepts_neutral_path() {
        let mut sb = fresh();
        assert!(sb
            .bind_rw(Path::new("/Users/me/projects/myrepo"), Path::new("/work"))
            .is_ok());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn env_policy_round_trips_through_setter() {
        let mut sb = fresh();
        sb.env(EnvPolicy::Pass(vec!["PATH".into(), "HOME".into()]));
        match &sb.env {
            EnvPolicy::Pass(v) => assert_eq!(v.len(), 2),
            other => panic!("got {other:?}"),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn limits_round_trip() {
        let mut sb = fresh();
        sb.limits(ResourceLimits {
            memory_bytes: Some(4 << 30),
            cpu_quota_ms_per_sec: Some(2000),
            ..Default::default()
        });
        assert_eq!(sb.limits.memory_bytes, Some(4 << 30));
        assert_eq!(sb.limits.cpu_quota_ms_per_sec, Some(2000));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn network_default_is_none_then_brokered_sticks() {
        let mut sb = fresh();
        assert!(matches!(sb.network, NetPolicy::None));
        sb.network(NetPolicy::Brokered {
            socket: PathBuf::from("/run/vibe-broker.sock"),
            policy_id: "test".into(),
        });
        assert!(matches!(sb.network, NetPolicy::Brokered { .. }));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn shutdown_is_noop_in_skeleton() {
        let sb = Box::new(fresh());
        sb.shutdown().unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn builder_methods_populate_runtime_state() {
        let sb = FirecrackerSandbox::new()
            .unwrap()
            .firecracker_bin(PathBuf::from("/usr/local/bin/firecracker"))
            .kernel_image(PathBuf::from("/var/lib/vibe/vmlinux"))
            .rootfs_image(PathBuf::from("/var/lib/vibe/rootfs.ext4"));
        assert_eq!(
            sb.runtime.firecracker_bin.as_deref(),
            Some(Path::new("/usr/local/bin/firecracker"))
        );
        assert_eq!(
            sb.runtime.kernel_image.as_deref(),
            Some(Path::new("/var/lib/vibe/vmlinux"))
        );
        assert_eq!(
            sb.runtime.rootfs_image.as_deref(),
            Some(Path::new("/var/lib/vibe/rootfs.ext4"))
        );
        assert_eq!(sb.runtime.vsock_cid, 3);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn new_refuses_on_non_linux_hosts() {
        let err = FirecrackerSandbox::new().expect_err("non-Linux must refuse");
        match err {
            SandboxError::TierUnsupported {
                tier: SandboxTier::Firecracker,
            } => {}
            other => panic!("expected TierUnsupported(Firecracker), got {:?}", other),
        }
    }

    #[test]
    fn is_supported_reflects_target_os() {
        assert_eq!(is_supported(), cfg!(target_os = "linux"));
    }
}
