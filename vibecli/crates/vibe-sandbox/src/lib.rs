//! Cross-platform sandbox trait + tier selection.
//!
//! See `docs/design/sandbox-tiers/README.md` for the full architecture.
//! Concrete tier implementations live in sibling crates:
//! - `vibe-sandbox-native` for Tier-0 (bwrap / sandbox-exec / AppContainer)
//! - `vibe-sandbox-wasi` for Tier-1
//! - `vibe-sandbox-hyperlight` for Tier-2 (Linux + Windows)
//! - `vibe-sandbox-firecracker` for Tier-3 (Linux)

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxTier {
    Native,
    Wasi,
    Hyperlight,
    Firecracker,
}

impl fmt::Display for SandboxTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SandboxTier::Native => "Native",
            SandboxTier::Wasi => "Wasi",
            SandboxTier::Hyperlight => "Hyperlight",
            SandboxTier::Firecracker => "Firecracker",
        })
    }
}

impl FromStr for SandboxTier {
    type Err = SandboxError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Native" | "native" => Ok(SandboxTier::Native),
            "Wasi" | "wasi" | "WASI" => Ok(SandboxTier::Wasi),
            "Hyperlight" | "hyperlight" => Ok(SandboxTier::Hyperlight),
            "Firecracker" | "firecracker" => Ok(SandboxTier::Firecracker),
            other => Err(SandboxError::UnknownTier(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindMode {
    Rw,
    Ro,
}

impl BindMode {
    pub fn allows_writes(self) -> bool {
        matches!(self, BindMode::Rw)
    }
    pub fn allows_reads(self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub enum NetPolicy {
    None,
    Brokered { socket: PathBuf, policy_id: String },
    Direct,
}

impl Default for NetPolicy {
    fn default() -> Self {
        NetPolicy::None
    }
}

#[derive(Debug, Clone)]
pub enum EnvPolicy {
    Clear,
    Pass(Vec<String>),
    Inherit { strip_secrets: bool },
}

impl Default for EnvPolicy {
    fn default() -> Self {
        EnvPolicy::Clear
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    pub cpu_quota_ms_per_sec: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub pids: Option<u32>,
    pub wall_clock: Option<Duration>,
    pub max_open_files: Option<u32>,
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("unknown sandbox tier: {0}")]
    UnknownTier(String),
    #[error("tier {tier} not supported on this platform")]
    TierUnsupported { tier: SandboxTier },
    #[error("sandbox setup failed: {0}")]
    Setup(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("operation not supported by tier: {0}")]
    NotSupported(&'static str),
}

pub type Result<T> = std::result::Result<T, SandboxError>;

pub trait Sandbox: Send + Sync {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()>;
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()>;
    fn env(&mut self, policy: EnvPolicy);
    fn limits(&mut self, limits: ResourceLimits);
    fn network(&mut self, policy: NetPolicy);
    fn spawn(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Child>;
    fn tier(&self) -> SandboxTier;
    fn shutdown(self: Box<Self>) -> Result<()>;
}

pub struct SelectOptions {
    pub host_supports_firecracker: bool,
    pub host_supports_hyperlight: bool,
    pub on_downgrade: Option<Box<dyn FnMut() + Send + 'static>>,
}

impl Default for SelectOptions {
    fn default() -> Self {
        SelectOptions {
            host_supports_firecracker: cfg!(target_os = "linux"),
            host_supports_hyperlight: cfg!(any(target_os = "linux", target_os = "windows")),
            on_downgrade: None,
        }
    }
}

pub struct Selection {
    sandbox: Box<dyn Sandbox>,
    downgraded: bool,
}

impl Selection {
    pub fn into_sandbox(self) -> Box<dyn Sandbox> {
        self.sandbox
    }
    pub fn downgraded(&self) -> bool {
        self.downgraded
    }
    pub fn tier(&self) -> SandboxTier {
        self.sandbox.tier()
    }
}

pub fn select(tier: SandboxTier, opts: &SelectOptions) -> Result<Selection> {
    let (effective, downgraded) = match tier {
        SandboxTier::Firecracker if !opts.host_supports_firecracker => (SandboxTier::Native, true),
        SandboxTier::Hyperlight if !opts.host_supports_hyperlight => (SandboxTier::Native, true),
        t => (t, false),
    };
    let sandbox = build_stub(effective)?;
    Ok(Selection { sandbox, downgraded })
}

/// Until concrete tier crates are wired in, every tier resolves to a stub
/// sandbox that records its tier identity. Real implementations land in
/// sibling crates and override this through a registry pattern.
fn build_stub(tier: SandboxTier) -> Result<Box<dyn Sandbox>> {
    Ok(Box::new(StubSandbox {
        tier,
        rw: vec![],
        ro: vec![],
        net: NetPolicy::None,
        env: EnvPolicy::Clear,
        limits: ResourceLimits::default(),
    }))
}

struct StubSandbox {
    tier: SandboxTier,
    rw: Vec<(PathBuf, PathBuf)>,
    ro: Vec<(PathBuf, PathBuf)>,
    net: NetPolicy,
    env: EnvPolicy,
    limits: ResourceLimits,
}

impl Sandbox for StubSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.rw.push((host.to_owned(), guest.to_owned()));
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
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
        self.net = policy;
    }
    fn spawn(&self, _cmd: &OsStr, _args: &[&OsStr]) -> Result<Child> {
        Err(SandboxError::NotSupported(
            "stub sandbox cannot spawn — install a tier crate",
        ))
    }
    fn tier(&self) -> SandboxTier {
        self.tier
    }
    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_round_trip_via_string() {
        for &t in &[
            SandboxTier::Native,
            SandboxTier::Wasi,
            SandboxTier::Hyperlight,
            SandboxTier::Firecracker,
        ] {
            let s = t.to_string();
            let parsed: SandboxTier = s.parse().unwrap();
            assert_eq!(parsed, t);
        }
    }

    #[test]
    fn unknown_tier_errors() {
        assert!("Bogus".parse::<SandboxTier>().is_err());
    }

    #[test]
    fn bind_mode_semantics() {
        assert!(BindMode::Rw.allows_writes());
        assert!(!BindMode::Ro.allows_writes());
        assert!(BindMode::Rw.allows_reads());
        assert!(BindMode::Ro.allows_reads());
    }

    #[test]
    fn net_policy_default_is_none() {
        assert!(matches!(NetPolicy::default(), NetPolicy::None));
    }

    #[test]
    fn select_native_returns_native() {
        let sb = select(SandboxTier::Native, &SelectOptions::default()).unwrap();
        assert_eq!(sb.tier(), SandboxTier::Native);
        assert!(!sb.downgraded());
    }

    #[test]
    fn select_firecracker_downgrades_when_unsupported() {
        let opts = SelectOptions {
            host_supports_firecracker: false,
            host_supports_hyperlight: false,
            on_downgrade: None,
        };
        let sb = select(SandboxTier::Firecracker, &opts).unwrap();
        assert_eq!(sb.tier(), SandboxTier::Native);
        assert!(sb.downgraded());
    }

    #[test]
    fn select_hyperlight_downgrades_when_unsupported() {
        let opts = SelectOptions {
            host_supports_firecracker: false,
            host_supports_hyperlight: false,
            on_downgrade: None,
        };
        let sb = select(SandboxTier::Hyperlight, &opts).unwrap();
        assert_eq!(sb.tier(), SandboxTier::Native);
        assert!(sb.downgraded());
    }
}
