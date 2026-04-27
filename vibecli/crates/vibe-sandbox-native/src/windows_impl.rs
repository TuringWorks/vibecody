//! Windows Tier-0 backend: AppContainer + Restricted Token + Job Object.
//!
//! v1 ships the type surface and config builder. The kernel-side wiring
//! (CreateAppContainerProfile / CreateProcessAsUser into AppContainer /
//! AssignProcessToJobObject) lands in slice N3.2 against the `windows`
//! crate. v1's `spawn` returns NotSupported until that lands.
//!
//! See `docs/design/sandbox-tiers/01-native-tier.md`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Child;

use vibe_sandbox::{
    EnvPolicy, NetPolicy, ResourceLimits, Result, Sandbox, SandboxError, SandboxTier,
};

#[derive(Debug, Default)]
pub struct WindowsSandbox {
    rw: Vec<(PathBuf, PathBuf)>,
    ro: Vec<(PathBuf, PathBuf)>,
    grant_inet_capability: bool,
    broker_pipe: Option<PathBuf>,
    env: EnvPolicy,
    limits: ResourceLimits,
}

impl WindowsSandbox {
    pub fn new() -> Result<Self> {
        Ok(WindowsSandbox::default())
    }

    /// Returns the AppContainer capabilities the policy currently requests.
    /// Defaults to empty (no internet, no enterpriseAuth, no privateNetwork).
    pub fn capabilities(&self) -> Vec<&'static str> {
        let mut caps = Vec::new();
        if self.grant_inet_capability {
            caps.push("internetClient");
        }
        caps
    }

    /// Returns rw / ro path bindings as a flat audit list. The slice N3.2
    /// implementation iterates these to apply ACEs to each path.
    pub fn paths(&self) -> Vec<(&PathBuf, &PathBuf, BindAccessMode)> {
        let mut out = Vec::with_capacity(self.rw.len() + self.ro.len());
        for (h, g) in &self.rw {
            out.push((h, g, BindAccessMode::ReadWrite));
        }
        for (h, g) in &self.ro {
            out.push((h, g, BindAccessMode::Read));
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindAccessMode {
    Read,
    ReadWrite,
}

impl Sandbox for WindowsSandbox {
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
                self.grant_inet_capability = false;
                self.broker_pipe = None;
            }
            NetPolicy::Brokered { socket, .. } => {
                self.grant_inet_capability = false;
                self.broker_pipe = Some(socket);
            }
            NetPolicy::Direct => {
                self.grant_inet_capability = true;
                self.broker_pipe = None;
            }
        }
    }

    fn spawn(&self, _cmd: &OsStr, _args: &[&OsStr]) -> Result<Child> {
        // Slice N3.2: wire CreateAppContainerProfile + CreateProcessAsUser
        // + AssignProcessToJobObject via the `windows` crate. Until then,
        // surface the gap as a typed error so callers fall back to a
        // subprocess without sandboxing on Windows-host CI.
        Err(SandboxError::NotSupported(
            "windows AppContainer spawn lands in slice N3.2",
        ))
    }

    fn tier(&self) -> SandboxTier {
        SandboxTier::Native
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

fn validate_path(p: &Path) -> Result<()> {
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(SandboxError::Setup(format!(
            "path traversal not allowed in sandbox path: {}",
            p.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_capabilities_empty() {
        let sb = WindowsSandbox::new().unwrap();
        assert!(sb.capabilities().is_empty());
    }

    #[test]
    fn direct_network_grants_inet_capability() {
        let mut sb = WindowsSandbox::new().unwrap();
        sb.network(NetPolicy::Direct);
        assert!(sb.capabilities().contains(&"internetClient"));
    }

    #[test]
    fn brokered_network_does_not_grant_inet_capability() {
        let mut sb = WindowsSandbox::new().unwrap();
        sb.network(NetPolicy::Brokered {
            socket: PathBuf::from(r"\\.\pipe\vibe-broker"),
            policy_id: "skill:test".into(),
        });
        assert!(!sb.capabilities().contains(&"internetClient"));
        assert!(sb.broker_pipe.is_some());
    }

    #[test]
    fn paths_reports_rw_and_ro() {
        let mut sb = WindowsSandbox::new().unwrap();
        sb.bind_rw(Path::new(r"C:\Users\me\repo"), Path::new(r"C:\work")).unwrap();
        sb.bind_ro(Path::new(r"C:\readonly"), Path::new(r"C:\readonly")).unwrap();
        let paths = sb.paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].2, BindAccessMode::ReadWrite);
        assert_eq!(paths[1].2, BindAccessMode::Read);
    }

    #[test]
    fn validate_path_rejects_traversal() {
        assert!(validate_path(Path::new(r"C:\Users\..\Windows")).is_err());
    }

    #[test]
    fn tier_is_native() {
        let sb = WindowsSandbox::new().unwrap();
        assert_eq!(sb.tier(), SandboxTier::Native);
    }
}
