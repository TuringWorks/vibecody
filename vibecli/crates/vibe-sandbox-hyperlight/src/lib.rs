//! Tier-2 (Hyperlight hypervisor partition) Sandbox impl for VibeCody.
//!
//! See `docs/design/sandbox-tiers/04-hyperlight-tier.md` for the full
//! design. This crate is Linux + Windows only — on macOS the public
//! constructor returns `SandboxError::TierUnsupported`, and the
//! daemon's tier-selection layer transparently downgrades to
//! Tier-1 (Wasmtime in-process) per
//! `docs/design/sandbox-tiers/README.md` §"Tier downgrade emits a
//! structured warning event."
//!
//! ## Sequencing
//!
//! This slice (T2.0) ships the **state-tracking skeleton** — mirrors
//! the Firecracker F0 slice. The Hyperlight runtime itself
//! (Microsoft's `hyperlight-host` crate, hypervisor partition
//! lifecycle) lands in T2.1.B.
//!
//! Hyperlight is fundamentally a *WASM-extension* backend, not a
//! shell-command one. The Sandbox trait's `spawn(cmd, args) -> Child`
//! method is therefore semantically a poor fit; this crate
//! implements the trait to keep the unified API but `spawn()`
//! returns `SandboxError::NotSupported` permanently. The intended
//! integration is via the `vibe-extensions` crate which speaks a
//! different host-function ABI — covered in slice T2.1.A.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Child;

use vibe_sandbox::{
    BindMode, EnvPolicy, NetPolicy, ResourceLimits, Result, Sandbox, SandboxError, SandboxTier,
};

/// Tier-2 Hyperlight sandbox.
///
/// State-tracking only in slice T2.0 — see crate-level docs.
#[derive(Debug)]
pub struct HyperlightSandbox {
    /// Read-write and read-only bind mounts. Hyperlight guests don't
    /// have a kernel filesystem, but the binds are still tracked so
    /// the future `vibe-extensions` integration knows which preopen
    /// directories to project as WASI capabilities.
    binds: Vec<(PathBuf, PathBuf, BindMode)>,
    env: EnvPolicy,
    limits: ResourceLimits,
    network: NetPolicy,
    /// Linux/Windows-only runtime state. Populated by builder methods
    /// in slice T2.1.B.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    runtime: HostRuntimeState,
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[allow(dead_code)]
    runtime: HostRuntimeState,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Default)]
struct HostRuntimeState {
    /// Maximum guest physical memory in bytes. Hyperlight partitions
    /// are typically tens of KB; default 64 KB.
    pub guest_memory_bytes: u64,
    /// Maximum host time the guest is allowed to run before the
    /// partition is torn down. Defaults to 250 ms — chosen to be
    /// large enough for a typical extension call, small enough to
    /// detect a runaway guest.
    pub call_timeout_ms: u32,
    /// Allow nested kernel virtualization (e.g. KVM-on-KVM). Default
    /// off; some CI hosts need it on to support nested test runs.
    pub allow_nested: bool,
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[derive(Debug, Default)]
#[allow(dead_code)]
struct HostRuntimeState;

impl HyperlightSandbox {
    /// Construct a new Hyperlight sandbox skeleton.
    ///
    /// Linux / Windows: state-tracking sandbox; `spawn()` always
    /// refuses (see crate docs — Hyperlight is for WASM, not
    /// shell). The future `vibe-extensions` integration consumes
    /// this via a different ABI.
    ///
    /// macOS: returns `SandboxError::TierUnsupported`.
    pub fn new() -> Result<Self> {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            Ok(Self {
                binds: Vec::new(),
                env: EnvPolicy::Clear,
                limits: ResourceLimits::default(),
                network: NetPolicy::None,
                runtime: HostRuntimeState {
                    guest_memory_bytes: 64 * 1024,
                    call_timeout_ms: 250,
                    allow_nested: false,
                },
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            Err(SandboxError::TierUnsupported {
                tier: SandboxTier::Hyperlight,
            })
        }
    }

    /// Set the guest's maximum physical memory in bytes.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn guest_memory_bytes(mut self, bytes: u64) -> Self {
        self.runtime.guest_memory_bytes = bytes;
        self
    }

    /// Set the per-call wall-clock timeout in milliseconds.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn call_timeout_ms(mut self, ms: u32) -> Self {
        self.runtime.call_timeout_ms = ms;
        self
    }

    /// Read-only view of the accumulated bind list.
    pub fn binds(&self) -> &[(PathBuf, PathBuf, BindMode)] {
        &self.binds
    }
}

impl Sandbox for HyperlightSandbox {
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
        // By design — Hyperlight is for WASM extensions, not shell.
        // The future integration goes through the `vibe-extensions`
        // crate's host-function ABI, not `Child`.
        Err(SandboxError::NotSupported(
            "Hyperlight runs WASM extensions, not OS processes — \
             integrate via vibe-extensions::call() instead",
        ))
    }

    fn tier(&self) -> SandboxTier {
        SandboxTier::Hyperlight
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        // T2.1.B will tear down the hypervisor partition. Skeleton
        // has no live runtime — shutdown is a no-op.
        Ok(())
    }
}

/// Reject bind sources that traverse a credential directory. Same
/// deny-list parity as Tier-0 native + Tier-3 Firecracker.
fn validate_bind_path(host: &Path) -> Result<()> {
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
                        "refusing to bind credential directory '{}' into Hyperlight guest",
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
                    "refusing to bind credential file '{}' into Hyperlight guest",
                    denied
                )));
            }
        }
    }
    Ok(())
}

/// Whether the current host supports Hyperlight.
pub fn is_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "windows"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn fresh() -> HyperlightSandbox {
        HyperlightSandbox::new().expect("supported host must construct ok")
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn new_returns_hyperlight_tier() {
        assert_eq!(fresh().tier(), SandboxTier::Hyperlight);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn spawn_always_refuses_by_design() {
        let sb = fresh();
        let cmd = std::ffi::OsString::from("/bin/echo");
        let arg = std::ffi::OsString::from("hi");
        let err = sb.spawn(&cmd, &[&arg]).expect_err("must refuse");
        match err {
            SandboxError::NotSupported(msg) => {
                assert!(
                    msg.to_lowercase().contains("wasm"),
                    "refusal message should mention WASM: got '{msg}'"
                );
            }
            other => panic!("expected NotSupported, got {:?}", other),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn default_guest_memory_is_64_kb() {
        let sb = fresh();
        assert_eq!(sb.runtime.guest_memory_bytes, 64 * 1024);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn default_call_timeout_is_250_ms() {
        let sb = fresh();
        assert_eq!(sb.runtime.call_timeout_ms, 250);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn builder_methods_override_defaults() {
        let sb = HyperlightSandbox::new()
            .unwrap()
            .guest_memory_bytes(128 * 1024)
            .call_timeout_ms(500);
        assert_eq!(sb.runtime.guest_memory_bytes, 128 * 1024);
        assert_eq!(sb.runtime.call_timeout_ms, 500);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn bind_refuses_credential_segment() {
        let mut sb = fresh();
        let err = sb
            .bind_rw(Path::new("/home/me/.aws"), Path::new("/preopen/aws"))
            .expect_err("must refuse");
        assert!(matches!(err, SandboxError::Setup(_)));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn bind_refuses_credential_filename() {
        let mut sb = fresh();
        let err = sb
            .bind_rw(Path::new("/tmp/id_ed25519"), Path::new("/preopen/key"))
            .expect_err("must refuse");
        assert!(matches!(err, SandboxError::Setup(_)));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn bind_accepts_neutral_path() {
        let mut sb = fresh();
        sb.bind_ro(Path::new("/usr/local"), Path::new("/preopen/usr"))
            .unwrap();
        assert_eq!(sb.binds().len(), 1);
        assert!(matches!(sb.binds()[0].2, BindMode::Ro));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn env_and_limits_round_trip() {
        let mut sb = fresh();
        sb.env(EnvPolicy::Pass(vec!["LANG".into()]));
        sb.limits(ResourceLimits {
            wall_clock: Some(std::time::Duration::from_millis(100)),
            ..Default::default()
        });
        match &sb.env {
            EnvPolicy::Pass(v) => assert_eq!(v.len(), 1),
            other => panic!("got {other:?}"),
        }
        assert_eq!(
            sb.limits.wall_clock,
            Some(std::time::Duration::from_millis(100))
        );
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn shutdown_is_noop_in_skeleton() {
        Box::new(fresh()).shutdown().unwrap();
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[test]
    fn new_refuses_on_macos() {
        let err = HyperlightSandbox::new().expect_err("macOS must refuse");
        match err {
            SandboxError::TierUnsupported {
                tier: SandboxTier::Hyperlight,
            } => {}
            other => panic!("expected TierUnsupported(Hyperlight), got {:?}", other),
        }
    }

    #[test]
    fn is_supported_reflects_target_os() {
        assert_eq!(
            is_supported(),
            cfg!(any(target_os = "linux", target_os = "windows"))
        );
    }
}
